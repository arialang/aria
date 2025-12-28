// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use aria_compiler::{bc_reader::BytecodeReader, compile_from_source, module::CompiledModule};
use aria_parser::ast::{SourceBuffer, prettyprint::printout_accumulator::PrintoutAccumulator};
use haxby_opcodes::{BuiltinTypeId, Opcode, enum_case_attribs::CASE_HAS_PAYLOAD};
use std::sync::OnceLock;

use crate::{
    builtins::VmBuiltins,
    console::{Console, StdConsole},
    error::{
        dylib_load::{LoadResult, LoadStatus},
        exception::VmException,
        vm_error::{VmError, VmErrorReason},
    },
    frame::Frame,
    opcodes::prettyprint::opcode_prettyprint,
    runtime_module::RuntimeModule,
    runtime_value::{
        RuntimeValue,
        enumeration::{Enum, EnumCase},
        function::Function,
        isa::IsaCheckable,
        kind::RuntimeValueType,
        list::List,
        mixin::Mixin,
        object::Object,
        runtime_code_object::CodeObject,
        structure::Struct,
    },
    stack::Stack,
};

pub type ConsoleHandle = Rc<RefCell<dyn Console>>;

#[derive(Clone)]
pub struct VmOptions {
    pub tracing: bool,
    pub dump_stack: bool,
    pub vm_args: Vec<String>,
    pub console: ConsoleHandle,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            tracing: Default::default(),
            dump_stack: Default::default(),
            vm_args: Default::default(),
            console: Rc::new(RefCell::new(StdConsole {})),
        }
    }
}

pub struct VirtualMachine {
    pub modules: HashMap<String, RuntimeModule>,
    pub options: VmOptions,
    pub builtins: VmBuiltins,
    pub import_stack: Stack<String>,
    pub imported_modules: HashMap<String, ModuleLoadInfo>,
    pub loaded_dylibs: HashMap<String, libloading::Library>,
}

impl VirtualMachine {
    pub fn console(&self) -> &ConsoleHandle {
        &self.options.console
    }

    fn load_version_into_builtins(self) -> Self {
        let aria_version = env!("CARGO_PKG_VERSION");
        assert!(!aria_version.is_empty());
        self.builtins
            .insert("ARIA_VERSION", RuntimeValue::String(aria_version.into()));
        self
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        let options = Default::default();
        VirtualMachine::with_options(options)
    }
}

impl VirtualMachine {
    pub fn with_options(options: VmOptions) -> Self {
        Self {
            modules: Default::default(),
            options,
            builtins: Default::default(),
            import_stack: Default::default(),
            imported_modules: Default::default(),
            loaded_dylibs: Default::default(),
        }
        .load_version_into_builtins()
    }
}

macro_rules! build_vm_error {
    ($reason: expr, $next: expr, $frame: expr, $idx: expr) => {{
        let lt = if let Some(lt) = $frame.get_line_table() {
            lt.get($idx as u16)
        } else {
            None
        };
        Err($crate::error::vm_error::VmError {
            reason: $reason,
            opcode: Some($next),
            loc: lt,
            backtrace: Default::default(),
        })
    }};
}

macro_rules! pop_or_err {
    ($next: expr, $frame: expr, $idx: expr) => {
        if let Some(val) = $frame.stack.try_pop() {
            val
        } else {
            return build_vm_error!(VmErrorReason::EmptyStack, $next, $frame, $idx);
        }
    };
}

pub type ExecutionResult<T = (), U = VmError> = Result<T, U>;

pub struct ModuleLoadInfo {
    pub module: RuntimeModule,
}

pub enum RunloopExit<T = ()> {
    Ok(T),
    Exception(crate::error::exception::VmException),
}

impl RunloopExit {
    pub fn throw_object(value: Object) -> Self {
        Self::Exception(VmException {
            value: RuntimeValue::Object(value),
            backtrace: Default::default(),
        })
    }

    pub fn throw_struct(struk: &Struct, values: &[(&str, RuntimeValue)]) -> Self {
        let object = Object::new(struk);
        for value in values {
            object.write(value.0, value.1.clone());
        }

        Self::throw_object(object)
    }
}

enum OpcodeRunExit {
    Continue,
    Return,
    Exception(VmException),
}

macro_rules! binop_eval {
    ( ($op_expr: expr), $next: expr, $frame: expr, $op_idx: expr) => {
        match $op_expr {
            crate::runtime_value::OperatorEvalOutcome::Ok(val) => $frame.stack.push(val),
            crate::runtime_value::OperatorEvalOutcome::Exception(e) => {
                return Ok(OpcodeRunExit::Exception(e));
            }
            crate::runtime_value::OperatorEvalOutcome::Error(err) => {
                if err.loc.is_some() {
                    return Err(err);
                } else {
                    return build_vm_error!(err.reason, $next, $frame, $op_idx);
                }
            }
        }
    };
}

macro_rules! unaryop_eval {
    ( ($op_expr: expr), $next: expr, $frame: expr, $op_idx: expr) => {
        match $op_expr {
            crate::runtime_value::OperatorEvalOutcome::Ok(val) => $frame.stack.push(val),
            crate::runtime_value::OperatorEvalOutcome::Exception(e) => {
                return Ok(OpcodeRunExit::Exception(e));
            }
            crate::runtime_value::OperatorEvalOutcome::Error(err) => {
                if err.loc.is_some() {
                    return Err(err);
                } else {
                    return build_vm_error!(err.reason, $next, $frame, $op_idx);
                }
            }
        }
    };
}

fn get_lib_path(lib_name: &str) -> PathBuf {
    let exe_path = std::env::current_exe().expect("failed to get current exe path");
    let exe_dir = exe_path.parent().expect("failed to get exe directory");
    let lib_name = libloading::library_filename(lib_name);
    exe_dir.join(lib_name)
}

fn unique_insert<T>(vec: &mut Vec<T>, item: T) -> &Vec<T>
where
    T: PartialEq,
{
    if !vec.contains(&item) {
        vec.push(item);
    }
    vec
}

impl VirtualMachine {
    fn get_system_import_paths() -> Vec<PathBuf> {
        if let Ok(env_var) = std::env::var("ARIA_LIB_DIR") {
            let mut paths = Vec::new();
            for candidate_dir in std::env::split_paths(env_var.as_str()) {
                if candidate_dir.exists() && candidate_dir.is_dir() {
                    paths.push(candidate_dir);
                }
            }

            if !paths.is_empty() {
                return paths;
            }
        }

        if let Ok(exe_path) = std::env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            let lib_aria_path = exe_dir.join("lib");
            if lib_aria_path.join("aria").is_dir() {
                return vec![lib_aria_path];
            }

            if let Some(exe_parent_dir) = exe_dir.parent() {
                let lib_aria_path = exe_parent_dir.join("lib");
                if lib_aria_path.join("aria").is_dir() {
                    return vec![lib_aria_path];
                }
            }
        }

        let version = env!("CARGO_PKG_VERSION");

        #[cfg(target_os = "linux")]
        {
            let system_lib_path = PathBuf::from(format!("/usr/local/aria{}/lib", version));
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }

            let system_lib_path = PathBuf::from("/usr/local/aria/lib");
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }

            let system_lib_path = PathBuf::from(format!("/usr/lib/aria{}", version));
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }

            let system_lib_path = PathBuf::from("/usr/lib/aria");
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }
        }

        #[cfg(target_os = "macos")]
        {
            let system_lib_path = PathBuf::from(format!("/opt/homebrew/opt/aria{}/lib", version));
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }

            let system_lib_path = PathBuf::from("/opt/homebrew/opt/aria/lib");
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }

            let version = env!("CARGO_PKG_VERSION");
            let system_lib_path = PathBuf::from(format!("/usr/local/opt/aria{}/lib", version));
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }

            let system_lib_path = PathBuf::from("/usr/local/opt/aria/lib");
            if system_lib_path.join("aria").is_dir() {
                return vec![system_lib_path];
            }
        }

        Vec::new()
    }

    pub fn get_aria_library_paths() -> &'static Vec<PathBuf> {
        static ARIA_LIBRARY_PATHS: OnceLock<Vec<PathBuf>> = OnceLock::new();

        ARIA_LIBRARY_PATHS.get_or_init(|| {
            let mut paths = Self::get_system_import_paths();

            if let Ok(env_var) = std::env::var("ARIA_LIB_DIR_EXTRA") {
                for candidate_dir in std::env::split_paths(env_var.as_str()) {
                    if candidate_dir.exists() && candidate_dir.is_dir() {
                        paths.push(candidate_dir);
                    }
                }
            }

            let mut ret_paths = vec![];
            for path in paths {
                if let Ok(can) = std::fs::canonicalize(path) {
                    unique_insert(&mut ret_paths, can);
                }
            }

            ret_paths
        })
    }

    fn try_get_import_path_from_name(
        aria_lib_dir: impl AsRef<Path>,
        ipath: &str,
    ) -> Option<PathBuf> {
        let mut module_path = aria_lib_dir.as_ref().to_path_buf();
        module_path.push(ipath);
        if module_path.exists() {
            Some(module_path)
        } else {
            None
        }
    }

    fn resolve_import_path_to_path(
        ipath: &str,
        widget_root_path: Option<&PathBuf>,
    ) -> Result<PathBuf, VmErrorReason> {
        if let Some(ipath) = ipath.strip_prefix("widget.") {
            return if let Some(widget_root_path) = widget_root_path {
                let ipath = format!("{}.aria", ipath.replace(".", "/"));
                Self::try_get_import_path_from_name(widget_root_path, &ipath).ok_or_else(|| {
                    VmErrorReason::ImportNotAvailable(
                        ipath.to_owned(),
                        "import not found in widget".to_owned(),
                    )
                })
            } else {
                Err(VmErrorReason::ImportNotAvailable(
                    ipath.to_owned(),
                    "tried to import from widget without a widget.json".to_owned(),
                ))
            };
        }

        let ipath = format!("{}.aria", ipath.replace(".", "/"));

        let aria_lib_dirs = VirtualMachine::get_aria_library_paths();
        for aria_lib_dir in aria_lib_dirs {
            if let Some(path) = Self::try_get_import_path_from_name(aria_lib_dir, &ipath) {
                return Ok(path);
            }
        }

        Err(VmErrorReason::ImportNotAvailable(
            ipath.to_owned(),
            "no such path".to_owned(),
        ))
    }

    fn create_import_model_from_path(
        module: &RuntimeModule,
        ipath: &str,
        leaf: RuntimeValue,
    ) -> Result<RuntimeValue, VmErrorReason> {
        let components = ipath.split(".").collect::<Vec<_>>();
        if components.len() == 1 {
            let cmp_last = components[0];
            module.store_named_value(cmp_last, leaf.clone());
            return Ok(leaf);
        }

        let cmp0 = components[0];
        let root = {
            match module.load_named_value(cmp0) {
                Some(cmp0_obj) => match cmp0_obj.as_enum() {
                    Some(s) => s.clone(),
                    _ => {
                        return Err(VmErrorReason::UnexpectedType);
                    }
                },
                _ => {
                    let cmp0_struct = Enum::new(cmp0);
                    let cmp0_val = RuntimeValue::Type(RuntimeValueType::Enum(cmp0_struct.clone()));
                    module.store_named_value(cmp0, cmp0_val);
                    cmp0_struct
                }
            }
        };

        let mut current_struct = root.clone();

        fn get_or_create_empty_enum(
            current_struct: &Enum,
            name: &str,
        ) -> Result<Enum, VmErrorReason> {
            match current_struct.load_named_value(name) {
                Some(existing_val) => match existing_val.as_enum() {
                    Some(s) => Ok(s.clone()),
                    _ => Err(VmErrorReason::UnexpectedType),
                },
                None => {
                    let new_struct = Enum::new(name);
                    let new_val = RuntimeValue::Type(RuntimeValueType::Enum(new_struct.clone()));
                    current_struct.store_named_value(name, new_val);
                    Ok(new_struct)
                }
            }
        }

        for cmp in components.iter().take(components.len() - 1).skip(1) {
            current_struct = get_or_create_empty_enum(&current_struct, cmp)?;
        }

        let cmp_last = components.last().unwrap();
        current_struct.store_named_value(cmp_last, leaf.clone());

        Ok(RuntimeValue::Type(RuntimeValueType::Enum(root)))
    }

    pub fn load_into_module(
        &mut self,
        name: &str,
        r_mod: RuntimeModule,
    ) -> ExecutionResult<RunloopExit<ModuleLoadInfo>> {
        if !name.is_empty() {
            self.modules.insert(name.to_owned(), r_mod.clone());
        }

        let entry_cm = r_mod.get_compiled_module();
        let entry_cco = entry_cm.load_entry_code_object();
        let entry_co: CodeObject = Into::into(&entry_cco);
        let entry_f = Function::from_code_object(&entry_co, 0, &r_mod);
        let mut entry_frame: Frame = Default::default();

        let entry_result = entry_f.eval(0, &mut entry_frame, self, &Default::default(), true);
        match entry_result {
            Ok(ok) => match ok {
                crate::runtime_value::CallResult::Exception(e) => Ok(RunloopExit::Exception(e)),
                _ => Ok(RunloopExit::Ok(ModuleLoadInfo { module: r_mod })),
            },
            Err(err) => Err(err),
        }
    }

    pub fn load_module(
        &mut self,
        name: &str,
        entry_cm: CompiledModule,
    ) -> ExecutionResult<RunloopExit<ModuleLoadInfo>> {
        let r_mod = RuntimeModule::new(entry_cm);
        self.load_into_module(name, r_mod)
    }

    pub fn get_module_by_name(&self, name: &str) -> Option<RuntimeModule> {
        self.modules.get(name).cloned()
    }

    pub(crate) fn find_imported_module(&self, name: &str) -> Option<RuntimeModule> {
        self.imported_modules
            .get(name)
            .map(|mli| mli.module.clone())
    }

    pub fn inject_imported_module(&mut self, name: &str, module: RuntimeModule) {
        self.imported_modules
            .insert(name.to_owned(), ModuleLoadInfo { module });
    }

    pub fn execute_module(&mut self, m: &RuntimeModule) -> ExecutionResult<RunloopExit> {
        let main_f = match m.load_named_value("main") {
            Some(RuntimeValue::Function(f)) => f,
            _ => return Ok(RunloopExit::Ok(())),
        };

        let mut main_frame = Frame::default();

        let main_arity = main_f.arity();
        let req = main_arity.required;
        let opt = main_arity.optional;
        let va = main_f.varargs();
        let main_argc = if req == 0 && opt == 0 && !va {
            0
        } else if req == 1 && opt == 0 && !va {
            let cmdline_args = RuntimeValue::List(List::from(
                &self
                    .options
                    .vm_args
                    .iter()
                    .map(|arg| RuntimeValue::String(arg.as_str().into()))
                    .collect::<Vec<_>>(),
            ));
            main_frame.stack.push(cmdline_args);
            1
        } else if req == 0 && opt == 0 && va {
            self.options
                .vm_args
                .iter()
                .rev()
                .map(|arg| RuntimeValue::String(arg.as_str().into()))
                .for_each(|arg| main_frame.stack.push(arg));
            self.options.vm_args.len() as u8
        } else {
            return Err(VmErrorReason::InvalidMainSignature.into());
        };

        match main_f.eval(main_argc, &mut main_frame, self, &Default::default(), true)? {
            crate::runtime_value::CallResult::OkNoValue
            | crate::runtime_value::CallResult::Ok(_) => Ok(RunloopExit::Ok(())),
            crate::runtime_value::CallResult::Exception(e) => Ok(RunloopExit::Exception(e)),
        }
    }

    fn read_named_symbol(
        &self,
        module: &RuntimeModule,
        name: &str,
    ) -> Result<RuntimeValue, VmErrorReason> {
        match module.load_named_value(name) {
            Some(nv) => Ok(nv),
            _ => match self.builtins.load_named_value(name) {
                Some(nv) => Ok(nv),
                _ => Err(VmErrorReason::NoSuchIdentifier(name.to_owned())),
            },
        }
    }

    pub(crate) fn eval_bytecode_in_frame(
        &mut self,
        module: &RuntimeModule,
        bc: &[u8],
        target_frame: &mut Frame,
    ) -> ExecutionResult<RunloopExit> {
        let mut bc_reader = BytecodeReader::from(bc);
        self.runloop(&mut bc_reader, module, target_frame)
    }

    fn run_opcode(
        &mut self,
        next: Opcode,
        op_idx: usize,
        reader: &mut BytecodeReader,
        this_module: &RuntimeModule,
        frame: &mut Frame,
    ) -> ExecutionResult<OpcodeRunExit, VmError> {
        match next {
            Opcode::Nop => {}
            Opcode::Push(n) => {
                let ct = this_module.load_indexed_const(n);
                if let Some(ct) = ct {
                    frame.stack.push(RuntimeValue::from(&ct));
                }
            }
            Opcode::Push0 => frame.stack.push(RuntimeValue::Integer(0.into())),
            Opcode::Push1 => frame.stack.push(RuntimeValue::Integer(1.into())),
            Opcode::PushTrue => frame.stack.push(RuntimeValue::Boolean(true.into())),
            Opcode::PushFalse => frame.stack.push(RuntimeValue::Boolean(false.into())),
            Opcode::PushBuiltinTy(n) => match self.builtins.get_builtin_type_by_id(n) {
                Some(bty) => {
                    frame.stack.push(RuntimeValue::Type(bty));
                }
                _ => {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            },
            Opcode::PushRuntimeValue(n) => match n {
                haxby_opcodes::BuiltinValueId::ThisModule => {
                    frame.stack.push(RuntimeValue::Module(this_module.clone()))
                }
            },
            Opcode::Pop => {
                pop_or_err!(next, frame, op_idx);
            }
            Opcode::Dup => {
                if let Some(val) = frame.stack.peek() {
                    let val = val.clone();
                    frame.stack.push(val);
                }
            }
            Opcode::Swap => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                frame.stack.push(x);
                frame.stack.push(y);
            }
            Opcode::Copy(n) => {
                let val = frame.stack.peek_at_offset(n as usize);
                if let Some(val) = val {
                    let val = val.clone();
                    frame.stack.push(val);
                }
            }
            Opcode::Add => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Integer(b + a));
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(b + &a.to_fp()));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(&b.to_fp() + a));
                } else if let (RuntimeValue::String(a), RuntimeValue::String(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::String(b + a));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(b + a))
                } else {
                    binop_eval!(
                        (RuntimeValue::add(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::Sub => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Integer(b - a));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(b - a))
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(b - &a.to_fp()));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(&b.to_fp() - a));
                } else {
                    binop_eval!(
                        (RuntimeValue::sub(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::Mul => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Integer(b * a));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(b * a))
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(b * &a.to_fp()));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Float(&b.to_fp() * a));
                } else {
                    binop_eval!(
                        (RuntimeValue::mul(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::Div => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    if a.raw_value() == 0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Integer(b / a));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    if a.raw_value() == 0.0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Float(b / a))
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    if a.raw_value() == 0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Float(b / &a.to_fp()));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    if a.raw_value() == 0.0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Float(&b.to_fp() / a));
                } else {
                    binop_eval!(
                        (RuntimeValue::div(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::Rem => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    if a.raw_value() == 0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Integer(b % a));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    if a.raw_value() == 0.0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Float(b % a))
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    if a.raw_value() == 0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Float(b % &a.to_fp()))
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    if a.raw_value() == 0.0 {
                        return build_vm_error!(VmErrorReason::DivisionByZero, next, frame, op_idx);
                    }
                    frame.stack.push(RuntimeValue::Float(&b.to_fp() % a))
                } else {
                    binop_eval!(
                        (RuntimeValue::rem(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::Neg => {
                let n = pop_or_err!(next, frame, op_idx);
                if let RuntimeValue::Integer(i) = &n {
                    frame.stack.push(RuntimeValue::Integer(-i));
                } else if let RuntimeValue::Float(i) = &n {
                    frame.stack.push(RuntimeValue::Float(-i));
                } else {
                    unaryop_eval!((RuntimeValue::neg(&n, frame, self)), next, frame, op_idx)
                }
            }
            Opcode::ShiftLeft => {
                let by = pop_or_err!(next, frame, op_idx);
                let n = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(n), RuntimeValue::Integer(by)) = (&n, &by) {
                    frame.stack.push(RuntimeValue::Integer(n << by));
                } else {
                    binop_eval!(
                        (RuntimeValue::leftshift(&n, &by, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::ShiftRight => {
                let by = pop_or_err!(next, frame, op_idx);
                let n = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(n), RuntimeValue::Integer(by)) = (&n, &by) {
                    frame.stack.push(RuntimeValue::Integer(n >> by));
                } else {
                    binop_eval!(
                        (RuntimeValue::rightshift(&n, &by, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::Not => {
                let b = pop_or_err!(next, frame, op_idx);
                if let RuntimeValue::Boolean(b) = b {
                    frame.stack.push(RuntimeValue::Boolean(!b));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::Equal => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                let eq_result =
                    RuntimeValue::Boolean(Into::into(RuntimeValue::equals(&y, &x, frame, self)));
                frame.stack.push(eq_result);
            }
            Opcode::LessThan => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b < a)));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b < a)));
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(*b < a.to_fp())));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(b.to_fp() < *a)));
                } else {
                    binop_eval!(
                        (RuntimeValue::less_than(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::LessThanEqual => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b <= a)));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b <= a)));
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(*b <= a.to_fp())));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(b.to_fp() <= *a)));
                } else {
                    binop_eval!(
                        (RuntimeValue::less_than_equal(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::GreaterThan => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b > a)));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b > a)));
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(*b > a.to_fp())));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(b.to_fp() > *a)));
                } else {
                    binop_eval!(
                        (RuntimeValue::greater_than(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::GreaterThanEqual => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b >= a)));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(Into::into(b >= a)));
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Float(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(*b >= a.to_fp())));
                } else if let (RuntimeValue::Float(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame
                        .stack
                        .push(RuntimeValue::Boolean(Into::into(b.to_fp() >= *a)));
                } else {
                    binop_eval!(
                        (RuntimeValue::greater_than_equal(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::ReadLocal(n) => {
                let local = frame.locals[n as usize].val.clone();
                frame.stack.push(local);
            }
            Opcode::ReadUplevel(n) => {
                if let Some(f) = &frame.func {
                    if let Some(bcf) = f.imp.as_bytecode_function() {
                        match bcf.read_uplevel(n) {
                            Some(ulv) => {
                                frame.stack.push(ulv);
                            }
                            _ => {
                                return build_vm_error!(
                                    VmErrorReason::UplevelOutOfBounds(n as usize),
                                    next,
                                    frame,
                                    op_idx
                                );
                            }
                        }
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::StoreUplevel(n) => {
                let x = pop_or_err!(next, frame, op_idx);
                if let Some(f) = x.as_function() {
                    if let Some(bcf) = f.imp.as_bytecode_function() {
                        let local = frame.locals[n as usize].val.clone();
                        bcf.store_uplevel(n, local);
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
                frame.stack.push(x);
            }
            Opcode::WriteLocal(n) => {
                let x = pop_or_err!(next, frame, op_idx);
                let local = &mut frame.locals[n as usize];
                if !local.ty.isa_check(&x, &self.builtins) {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                } else {
                    local.val = x;
                }
            }
            Opcode::TypedefLocal(n) => {
                let t = pop_or_err!(next, frame, op_idx);
                if let Ok(isa_check) = IsaCheckable::try_from(&t) {
                    frame.locals[n as usize].ty = isa_check;
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::ReadNamed(n) => {
                if let Some(ct) = this_module.load_indexed_const(n)
                    && let Some(sv) = ct.as_string()
                {
                    frame.stack.push(self.read_named_symbol(this_module, sv)?);
                }
            }
            Opcode::WriteNamed(n) => {
                let x = pop_or_err!(next, frame, op_idx);
                if let Some(ct) = this_module.load_indexed_const(n)
                    && let Some(sv) = ct.as_string()
                {
                    let write_result =
                        this_module.store_typechecked_named_value(sv, x, &self.builtins);
                    match write_result {
                        Ok(_) => {}
                        Err(e) => {
                            return build_vm_error!(e, next, frame, op_idx);
                        }
                    }
                }
            }
            Opcode::TypedefNamed(n) => {
                let t = pop_or_err!(next, frame, op_idx);
                if let Ok(t) = IsaCheckable::try_from(&t) {
                    if let Some(ct) = this_module.load_indexed_const(n)
                        && let Some(sv) = ct.as_string()
                    {
                        this_module.typedef_named_value(sv, t);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::ReadIndex(n) => {
                let mut indices = Vec::<_>::with_capacity(n as usize);
                for _ in 0..n {
                    let idx = pop_or_err!(next, frame, op_idx);
                    indices.insert(0, idx);
                }
                let cnt = pop_or_err!(next, frame, op_idx);
                match cnt.read_index(&indices, frame, self) {
                    Ok(crate::runtime_value::CallResult::OkNoValue)
                    | Ok(crate::runtime_value::CallResult::Ok(_)) => {}
                    Ok(crate::runtime_value::CallResult::Exception(e)) => {
                        return Ok(OpcodeRunExit::Exception(e));
                    }
                    Err(e) => {
                        return if e.loc.is_none() {
                            build_vm_error!(e.reason, next, frame, op_idx)
                        } else {
                            Err(e)
                        };
                    }
                }
            }
            Opcode::WriteIndex(n) => {
                let val = pop_or_err!(next, frame, op_idx);
                let mut indices = Vec::<_>::with_capacity(n as usize);
                for _ in 0..n {
                    let idx = pop_or_err!(next, frame, op_idx);
                    indices.insert(0, idx);
                }
                let cnt = pop_or_err!(next, frame, op_idx);
                match cnt.write_index(&indices, &val, frame, self) {
                    Ok(crate::runtime_value::CallResult::OkNoValue)
                    | Ok(crate::runtime_value::CallResult::Ok(_)) => {}
                    Ok(crate::runtime_value::CallResult::Exception(e)) => {
                        return Ok(OpcodeRunExit::Exception(e));
                    }
                    Err(e) => {
                        return if e.loc.is_none() {
                            build_vm_error!(e.reason, next, frame, op_idx)
                        } else {
                            Err(e)
                        };
                    }
                }
            }
            Opcode::ReadAttribute(n) => {
                let attrib_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };
                let val_obj = pop_or_err!(next, frame, op_idx);
                match val_obj.read_attribute(&attrib_name, &self.builtins) {
                    Ok(val) => {
                        frame.stack.push(val);
                    }
                    Err(err) => {
                        return build_vm_error!(
                            match err {
                                crate::runtime_value::AttributeError::NoSuchAttribute => {
                                    VmErrorReason::NoSuchIdentifier(attrib_name)
                                }
                                crate::runtime_value::AttributeError::InvalidFunctionBinding => {
                                    VmErrorReason::InvalidBinding
                                }
                                crate::runtime_value::AttributeError::ValueHasNoAttributes => {
                                    VmErrorReason::UnexpectedType
                                }
                            },
                            next,
                            frame,
                            op_idx
                        );
                    }
                }
            }
            Opcode::WriteAttribute(n) => {
                let val = pop_or_err!(next, frame, op_idx);
                let obj = pop_or_err!(next, frame, op_idx);
                let attr_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };
                match obj.write_attribute(&attr_name, val) {
                    Ok(_) => {}
                    Err(err) => {
                        return build_vm_error!(
                            match err {
                                crate::runtime_value::AttributeError::NoSuchAttribute => {
                                    VmErrorReason::NoSuchIdentifier(attr_name)
                                }
                                crate::runtime_value::AttributeError::InvalidFunctionBinding => {
                                    VmErrorReason::InvalidBinding
                                }
                                crate::runtime_value::AttributeError::ValueHasNoAttributes => {
                                    VmErrorReason::UnexpectedType
                                }
                            },
                            next,
                            frame,
                            op_idx
                        );
                    }
                }
            }
            Opcode::LogicalAnd => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Boolean(a), RuntimeValue::Boolean(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(a & b));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::LogicalOr => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Boolean(a), RuntimeValue::Boolean(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(a | b));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::Xor => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Boolean(a), RuntimeValue::Boolean(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Boolean(a ^ b));
                } else if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Integer(a ^ b));
                } else {
                    binop_eval!(
                        (RuntimeValue::xor(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::BitwiseAnd => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Integer(a & b));
                } else if let Ok(x) = IsaCheckable::try_from(&x)
                    && let Ok(y) = IsaCheckable::try_from(&y)
                {
                    frame.stack.push(RuntimeValue::TypeCheck(x & &y));
                } else {
                    binop_eval!(
                        (RuntimeValue::bitwise_and(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::BitwiseOr => {
                let x = pop_or_err!(next, frame, op_idx);
                let y = pop_or_err!(next, frame, op_idx);
                if let (RuntimeValue::Integer(a), RuntimeValue::Integer(b)) = (&x, &y) {
                    frame.stack.push(RuntimeValue::Integer(a | b));
                } else if let Ok(x) = IsaCheckable::try_from(&x)
                    && let Ok(y) = IsaCheckable::try_from(&y)
                {
                    frame.stack.push(RuntimeValue::TypeCheck(x | &y));
                } else {
                    binop_eval!(
                        (RuntimeValue::bitwise_or(&y, &x, frame, self)),
                        next,
                        frame,
                        op_idx
                    )
                }
            }
            Opcode::JumpTrue(n) => {
                let x = pop_or_err!(next, frame, op_idx);
                if let RuntimeValue::Boolean(v) = x {
                    if v.raw_value() {
                        reader.jump_to_index(n as usize);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::JumpFalse(n) => {
                let x = pop_or_err!(next, frame, op_idx);
                if let RuntimeValue::Boolean(v) = x {
                    if !v.raw_value() {
                        reader.jump_to_index(n as usize);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::Jump(n) => {
                reader.jump_to_index(n as usize);
            }
            Opcode::JumpIfArgSupplied(arg, dest) => {
                if frame.argc > arg {
                    reader.jump_to_index(dest as usize);
                }
            }
            Opcode::Call(argc) => {
                let x = pop_or_err!(next, frame, op_idx);
                match x.eval(argc, frame, self, false) {
                    Ok(crate::runtime_value::CallResult::OkNoValue)
                    | Ok(crate::runtime_value::CallResult::Ok(_)) => {}
                    Ok(crate::runtime_value::CallResult::Exception(e)) => {
                        return Ok(OpcodeRunExit::Exception(e));
                    }
                    Err(err) => {
                        if err.loc.is_some() {
                            return Err(err);
                        } else {
                            return build_vm_error!(err.reason, next, frame, op_idx);
                        }
                    }
                }
            }
            Opcode::Return => {
                return Ok(OpcodeRunExit::Return);
            }
            Opcode::TryEnter(offset) => {
                frame
                    .ctrl_blocks
                    .push(crate::frame::ControlBlock::Try(offset));
            }
            Opcode::TryExit => match frame.ctrl_blocks.try_pop() {
                Some(block) => match block {
                    crate::frame::ControlBlock::Try(_) => {}
                },
                _ => {
                    return build_vm_error!(VmErrorReason::EmptyStack, next, frame, op_idx);
                }
            },
            Opcode::Throw => {
                let ev = pop_or_err!(next, frame, op_idx);
                match frame.drop_to_first_try(self) {
                    Some(catch_offset) => {
                        reader.jump_to_index(catch_offset as usize);
                        frame.stack.push(ev);
                    }
                    None => {
                        let e = VmException::from_value(ev);
                        return Ok(OpcodeRunExit::Exception(e));
                    }
                };
            }
            Opcode::BuildList(n) => {
                let values = (0..n).map(|_| frame.stack.try_pop()).collect::<Vec<_>>();
                let list = List::default();
                for value in values.iter().rev() {
                    list.append(match value {
                        Some(x) => x.clone(),
                        None => {
                            return build_vm_error!(VmErrorReason::EmptyStack, next, frame, op_idx);
                        }
                    });
                }
                let list = RuntimeValue::List(list);
                frame.stack.push(list);
            }
            Opcode::BuildFunction(a) => {
                let val = pop_or_err!(next, frame, op_idx);
                if let Some(co) = val.as_code_object() {
                    let f = Function::from_code_object(co, a, this_module);
                    frame.stack.push(RuntimeValue::Function(f));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::BuildStruct => {
                let name = pop_or_err!(next, frame, op_idx);
                if let Some(name) = name.as_string() {
                    frame
                        .stack
                        .push(RuntimeValue::Type(RuntimeValueType::Struct(Struct::new(
                            &name.raw_value(),
                        ))));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::BuildEnum => {
                let name = pop_or_err!(next, frame, op_idx);
                if let Some(name) = name.as_string() {
                    frame
                        .stack
                        .push(RuntimeValue::Type(RuntimeValueType::Enum(Enum::new(
                            &name.raw_value(),
                        ))));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::BuildMixin => {
                let name = pop_or_err!(next, frame, op_idx);
                if let Some(name) = name.as_string() {
                    frame
                        .stack
                        .push(RuntimeValue::Mixin(Mixin::new(&name.raw_value())));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::IncludeMixin => {
                let mixin = pop_or_err!(next, frame, op_idx);
                let struk = pop_or_err!(next, frame, op_idx);

                if let (Some(mixin), Some(strukt)) = (mixin.as_mixin(), struk.as_struct()) {
                    strukt.include_mixin(mixin);
                } else if let (Some(mixin), Some(enumm)) = (mixin.as_mixin(), struk.as_enum()) {
                    enumm.include_mixin(mixin);
                } else if let (Some(mixin), Some(btt)) = (mixin.as_mixin(), struk.as_builtin_type())
                {
                    btt.include_mixin(mixin);
                } else if let (Some(src_mixin), Some(dst_mixin)) =
                    (mixin.as_mixin(), struk.as_mixin())
                {
                    dst_mixin.include_mixin(src_mixin);
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::BindMethod(a, n) => {
                let method = pop_or_err!(next, frame, op_idx);
                let struk = pop_or_err!(next, frame, op_idx);
                let new_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                if let (Some(x), Some(y)) = (method.as_code_object(), struk.as_struct()) {
                    let new_f = Function::from_code_object(x, a, this_module);
                    y.store_named_value(&new_name, RuntimeValue::Function(new_f));
                } else if let (Some(x), Some(y)) = (method.as_code_object(), struk.as_enum()) {
                    let new_f = Function::from_code_object(x, a, this_module);
                    y.store_named_value(&new_name, RuntimeValue::Function(new_f));
                } else if let (Some(x), Some(y)) = (method.as_code_object(), struk.as_mixin()) {
                    let new_f = Function::from_code_object(x, a, this_module);
                    y.store_named_value(&new_name, RuntimeValue::Function(new_f));
                } else if let (Some(x), Some(y)) =
                    (method.as_code_object(), struk.as_builtin_type())
                {
                    let new_f = Function::from_code_object(x, a, this_module);
                    y.write(&new_name, RuntimeValue::Function(new_f));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::BindCase(a, n) => {
                let payload_type = if (a & CASE_HAS_PAYLOAD) == CASE_HAS_PAYLOAD {
                    let t = pop_or_err!(next, frame, op_idx);
                    if let Ok(t) = IsaCheckable::try_from(&t) {
                        Some(t)
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    None
                };

                let enumm = pop_or_err!(next, frame, op_idx);

                let new_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                match enumm.as_enum() {
                    Some(enumm) => {
                        enumm.add_case(EnumCase {
                            name: new_name,
                            payload_type,
                        });
                    }
                    _ => {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                }
            }
            Opcode::NewEnumVal(a, n) => {
                let has_payload = (a & CASE_HAS_PAYLOAD) == CASE_HAS_PAYLOAD;
                let case_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                let enumm = pop_or_err!(next, frame, op_idx);
                match enumm.as_enum() {
                    Some(enumm) => {
                        let case = enumm.get_idx_of_case(&case_name);
                        let (cidx, case) = if let Some(case) = case {
                            (case, enumm.get_case_by_idx(case).unwrap())
                        } else {
                            return build_vm_error!(
                                VmErrorReason::NoSuchCase(case_name),
                                next,
                                frame,
                                op_idx
                            );
                        };
                        if case.payload_type.is_some() != has_payload {
                            return build_vm_error!(
                                VmErrorReason::UnexpectedType,
                                next,
                                frame,
                                op_idx
                            );
                        }
                        let payload = match &case.payload_type {
                            Some(pt) => {
                                let pv = pop_or_err!(next, frame, op_idx);
                                if !pt.isa_check(&pv, &self.builtins) {
                                    return build_vm_error!(
                                        VmErrorReason::UnexpectedType,
                                        next,
                                        frame,
                                        op_idx
                                    );
                                } else {
                                    Some(pv)
                                }
                            }
                            None => None,
                        };
                        let ev = enumm.make_value(cidx, payload).unwrap();
                        frame.stack.push(RuntimeValue::EnumValue(ev));
                    }
                    _ => {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                }
            }
            Opcode::EnumCheckIsCase(n) => {
                let case_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                let ev = pop_or_err!(next, frame, op_idx);
                if let Some(ev) = ev.as_enum_value() {
                    let ec = ev
                        .get_container_enum()
                        .get_case_by_idx(ev.get_case_index())
                        .unwrap();
                    frame
                        .stack
                        .push(RuntimeValue::Boolean((ec.name == case_name).into()));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::EnumTryExtractPayload => {
                let ev = pop_or_err!(next, frame, op_idx);
                if let Some(ev) = ev.as_enum_value() {
                    let p = ev.get_payload();
                    match p {
                        None => {
                            frame.stack.push(RuntimeValue::Boolean(false.into()));
                        }
                        Some(p) => {
                            frame.stack.push(p.clone());
                            frame.stack.push(RuntimeValue::Boolean(true.into()));
                        }
                    }
                } else {
                    frame.stack.push(RuntimeValue::Boolean(false.into()));
                }
            }
            Opcode::TryUnwrapProtocol(mode) => {
                let result_enum = if let Some(re) =
                    self.builtins.get_builtin_type_by_id(BuiltinTypeId::Result)
                    && let Some(re) = re.as_enum()
                {
                    re.clone()
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedVmState, next, frame, op_idx);
                };
                let maybe_enum = if let Some(re) =
                    self.builtins.get_builtin_type_by_id(BuiltinTypeId::Maybe)
                    && let Some(re) = re.as_enum()
                {
                    re.clone()
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedVmState, next, frame, op_idx);
                };

                let val = pop_or_err!(next, frame, op_idx);
                let ev = if let Some(ev) = val.as_enum_value() {
                    if ev.get_container_enum() == &maybe_enum
                        || ev.get_container_enum() == &result_enum
                    {
                        ev.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                let case_index = ev.get_case_index();
                match case_index {
                    0 => {
                        // Ok/Some
                        if let Some(case_value) = ev.get_payload() {
                            frame.stack.push(case_value.clone());
                        } else {
                            return build_vm_error!(
                                VmErrorReason::EnumWithoutPayload,
                                next,
                                frame,
                                op_idx
                            );
                        }
                    }
                    1 => {
                        // Err/None
                        match mode {
                            haxby_opcodes::try_unwrap_protocol_mode::PROPAGATE_ERROR => {
                                frame.stack.push(val.clone());
                                return Ok(OpcodeRunExit::Return); // implement a Return
                            }
                            haxby_opcodes::try_unwrap_protocol_mode::ASSERT_ERROR => {
                                return build_vm_error!(
                                    VmErrorReason::AssertFailed("force unwrap failed".to_string()),
                                    next,
                                    frame,
                                    op_idx
                                );
                            }
                            _ => {
                                // should never happen
                                return build_vm_error!(
                                    VmErrorReason::IncompleteInstruction,
                                    next,
                                    frame,
                                    op_idx
                                );
                            }
                        }
                    }
                    _ => {
                        // should never happen
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                }
            }
            Opcode::Isa => {
                let t = pop_or_err!(next, frame, op_idx);
                let val = pop_or_err!(next, frame, op_idx);
                if let Ok(isa_check) = IsaCheckable::try_from(&t) {
                    frame.stack.push(RuntimeValue::Boolean(
                        isa_check.isa_check(&val, &self.builtins).into(),
                    ));
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                }
            }
            Opcode::Assert(n) => {
                let assert_msg = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                let x = pop_or_err!(next, frame, op_idx);
                if let Some(b) = x.as_boolean() {
                    if !b.raw_value() {
                        return build_vm_error!(
                            VmErrorReason::AssertFailed(assert_msg),
                            next,
                            frame,
                            op_idx
                        );
                    }
                } else {
                    return build_vm_error!(
                        VmErrorReason::AssertFailed(assert_msg),
                        next,
                        frame,
                        op_idx
                    );
                }
            }
            Opcode::Halt => {
                return build_vm_error!(VmErrorReason::VmHalted, next, frame, op_idx);
            }
            Opcode::LoadDylib(n) => {
                let module = pop_or_err!(next, frame, op_idx);
                let module = match module.as_module() {
                    Some(m) => m,
                    None => {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                };

                let lib_name = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                // this means that one cannot use the same dylib for multiple modules!
                #[allow(clippy::map_entry)]
                if !self.loaded_dylibs.contains_key(&lib_name) {
                    unsafe {
                        let dylib_path = get_lib_path(&lib_name);
                        let dylib = match libloading::Library::new(&dylib_path) {
                            Ok(d) => d,
                            Err(e) => {
                                return build_vm_error!(
                                    VmErrorReason::ImportNotAvailable(
                                        dylib_path.into_os_string().into_string().unwrap(),
                                        e.to_string()
                                    ),
                                    next,
                                    frame,
                                    op_idx
                                );
                            }
                        };
                        let symbol: libloading::Symbol<
                            unsafe extern "C" fn(
                                *const VirtualMachine,
                                *const RuntimeModule,
                            ) -> LoadResult,
                        > = match dylib.get(b"dylib_haxby_inject") {
                            Ok(f) => f,
                            Err(e) => {
                                return build_vm_error!(
                                    VmErrorReason::ImportNotAvailable(
                                        dylib_path.into_os_string().into_string().unwrap(),
                                        e.to_string()
                                    ),
                                    next,
                                    frame,
                                    op_idx
                                );
                            }
                        };

                        let load_result = symbol(
                            self as *const VirtualMachine,
                            module as *const RuntimeModule,
                        );
                        if load_result.status == LoadStatus::Success {
                            self.loaded_dylibs.insert(lib_name, dylib);
                        } else {
                            let msg = load_result.into_rust_string();
                            return build_vm_error!(
                                VmErrorReason::ImportNotAvailable(lib_name, msg),
                                next,
                                frame,
                                op_idx
                            );
                        }
                    }
                }
            }
            Opcode::LiftModule => {
                let dest = pop_or_err!(next, frame, op_idx);
                let dest = if let Some(m) = dest.as_module() {
                    m
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };
                let src = pop_or_err!(next, frame, op_idx);
                let src = if let Some(m) = src.as_module() {
                    m
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };
                match dest.lift_all_symbols_from_other(src, self) {
                    Ok(_) => {}
                    Err(e) => {
                        return build_vm_error!(e, next, frame, op_idx);
                    }
                }
            }
            Opcode::Import(n) => {
                let ipath = if let Some(ct) = this_module.load_indexed_const(n) {
                    if let Some(sv) = ct.as_string() {
                        sv.clone()
                    } else {
                        return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                    }
                } else {
                    return build_vm_error!(VmErrorReason::UnexpectedType, next, frame, op_idx);
                };

                if let Some(mli) = self.imported_modules.get(&ipath) {
                    Self::create_import_model_from_path(
                        this_module,
                        &ipath,
                        RuntimeValue::Module(mli.module.clone()),
                    )?;

                    frame.stack.push(RuntimeValue::Module(mli.module.clone()));
                } else {
                    let import_path = match Self::resolve_import_path_to_path(
                        &ipath,
                        this_module.get_compiled_module().widget_root_path.as_ref(),
                    ) {
                        Ok(ipath) => ipath,
                        Err(err) => {
                            return build_vm_error!(err, next, frame, op_idx);
                        }
                    };

                    let sb = match SourceBuffer::from_path(&import_path) {
                        Ok(sb) => sb,
                        Err(_) => {
                            return build_vm_error!(
                                VmErrorReason::ImportNotAvailable(ipath, "no such file".to_owned()),
                                next,
                                frame,
                                op_idx
                            );
                        }
                    };

                    if self.import_stack.contains(&ipath) {
                        return build_vm_error!(
                            VmErrorReason::CircularImport(ipath),
                            next,
                            frame,
                            op_idx
                        );
                    } else {
                        self.import_stack.push(ipath.clone());
                    }

                    let c_module = match compile_from_source(&sb, &Default::default()) {
                        Ok(cm) => cm,
                        Err(ces) => {
                            let err_msg = ces
                                .iter()
                                .map(|x| format!("error: {x}"))
                                .collect::<Vec<_>>()
                                .join("\n");
                            assert!(ipath == self.import_stack.pop());
                            return build_vm_error!(
                                VmErrorReason::ImportNotAvailable(
                                    ipath,
                                    format!("module failed to compile: {err_msg}")
                                ),
                                next,
                                frame,
                                op_idx
                            );
                        }
                    };
                    let mli = match self.load_module(&sb.name, c_module)? {
                        RunloopExit::Ok(mli) => mli,
                        RunloopExit::Exception(e) => {
                            assert!(ipath == self.import_stack.pop());
                            return Ok(OpcodeRunExit::Exception(e));
                        }
                    };

                    Self::create_import_model_from_path(
                        this_module,
                        &ipath,
                        RuntimeValue::Module(mli.module.clone()),
                    )?;

                    assert!(ipath == self.import_stack.pop());

                    frame.stack.push(RuntimeValue::Module(mli.module.clone()));

                    self.imported_modules.insert(ipath.clone(), mli);
                };
            }
        }

        Ok(OpcodeRunExit::Continue)
    }

    fn runloop(
        &mut self,
        reader: &mut BytecodeReader,
        module: &RuntimeModule,
        frame: &mut Frame,
    ) -> ExecutionResult<RunloopExit, VmError> {
        loop {
            if self.options.tracing && self.options.dump_stack {
                frame.stack.dump();
            }
            let op_idx = reader.get_index();
            let next = match reader.read_opcode() {
                Ok(next) => next,
                Err(err) => {
                    return match err {
                        aria_compiler::bc_reader::DecodeError::EndOfStream => {
                            Ok(RunloopExit::Ok(()))
                        }
                        aria_compiler::bc_reader::DecodeError::InsufficientData => {
                            Err(VmErrorReason::IncompleteInstruction.into())
                        }
                        aria_compiler::bc_reader::DecodeError::UnknownOpcode(n) => {
                            Err(VmErrorReason::UnknownOpcode(n).into())
                        }
                        aria_compiler::bc_reader::DecodeError::UnknownOperand(a, b) => {
                            Err(VmErrorReason::InvalidVmOperand(a, b).into())
                        }
                    };
                }
            };
            if self.options.tracing {
                let poa = PrintoutAccumulator::default();
                let next = opcode_prettyprint(&next, module, poa).value();
                let wrote_lt = if let Some(lt) = frame.get_line_entry_at_pos(op_idx as u16) {
                    println!("{op_idx:05}: {next} --> {lt}");
                    true
                } else {
                    false
                };
                if !wrote_lt {
                    println!("{op_idx:05}: {next}");
                }
            }

            // some errors can be converted into exceptions, so reserve the right to postpone exception handling
            let mut need_handle_exception: Option<VmException> = None;

            match self.run_opcode(next, op_idx, reader, module, frame) {
                Ok(OpcodeRunExit::Continue) => {}
                Ok(OpcodeRunExit::Return) => {
                    return Ok(RunloopExit::Ok(()));
                }
                Ok(OpcodeRunExit::Exception(except)) => {
                    need_handle_exception = Some(except);
                }
                Err(x) => match VmException::from_vmerror(x, &self.builtins) {
                    Ok(exception) => {
                        need_handle_exception = Some(exception);
                    }
                    Err(err) => {
                        let err = if let Some(lt) = frame.get_line_entry_at_pos(op_idx as u16) {
                            let mut new_err = err.clone();
                            new_err.backtrace.push(lt);
                            new_err
                        } else {
                            err
                        };
                        return Err(err);
                    }
                },
            }

            if let Some(except) = need_handle_exception {
                except.fill_in_backtrace();
                match frame.drop_to_first_try(self) {
                    Some(o) => {
                        reader.jump_to_index(o as usize);
                        frame.stack.push(except.value);
                    }
                    None => {
                        let new_except =
                            if let Some(lt) = frame.get_line_entry_at_pos(op_idx as u16) {
                                except.thrown_at(lt)
                            } else {
                                except
                            };
                        return Ok(RunloopExit::Exception(new_except));
                    }
                }
            }
        }
    }
}
