# Aria: A Fresh, Safe, and Flexible Language for High-Level Development
[![Linux Build](https://github.com/arialang/aria/actions/workflows/linux_build_test.yml/badge.svg?branch=master)](https://github.com/arialang/aria/actions/workflows/linux_build_test.yml)
[![Mac Build](https://github.com/arialang/aria/actions/workflows/macos_build_test.yml/badge.svg?branch=master)](https://github.com/arialang/aria/actions/workflows/macos_build_test.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Docs](https://img.shields.io/badge/Docs-Available-blue.svg)](https://arialang.github.io/)
[![Contributors](https://img.shields.io/github/contributors/arialang/aria)](https://github.com/arialang/aria/graphs/contributors)

Please read [this post](https://arialang.github.io/blog/2026_02/post.html) for the latest update on Aria.

Welcome to Aria. Aria is a modern, dynamic scripting language. It is meant to be a "sweet spot" language, easy to pick-up and with a good balance between ease of use, safety, and flexibility. Aria's design choices help you build great programs quickly.

Aria has modern, safer error handling: Aria replaces unreliable `None` pointer checks with a modern, multi-tiered approach to error handling. By emphasizing algebraic data types (e.g. `Maybe`), Aria makes errors explicit, safer, and easier to manage, with fewer runtime surprises.

`null`, the [billion dollar mistake](https://softwareengineering.stackexchange.com/questions/413149/if-null-is-a-billion-dollar-mistake-what-is-the-solution-to-represent-a-non-ini) just does not exist in Aria, making code safer, easier to maintain and error handling more robust.

```aria
func main() {
    val x = Int.parse("abc123");
    match x {
        case Ok(val) => {
            println("Parsed value: {0}".format(val));
        },
        case Err(err) => {
            println("Failed to parse integer: {0}".format(err));
        }
    }
}
```

Aria is memory safe from the start: Aria’s virtual machine, built on the Rust ecosystem, ensures memory safety out of the box, protecting you from common pitfalls like data corruption and security vulnerabilities. This lets you focus on building without worrying about risks you cannot manage.

Memory safety protects your code from issues such as memory corruption and dangling pointers. These issues can be hard to track down in large, complex systems. Aria brings these guarantees right from the start, so you can focus on building, not debugging.

Aria is designed for flexibility: Whether you need (some) type checks, an intuitive module system, or want to avoid the complexity of inheritance while still working with a modern object-based design, Aria adapts to your needs. It provides just enough structure to keep your code clean, without the overhead.

Aria’s object-based design uses composition instead of inheritance, which removes a lot of complexity from the language. This makes Aria easier to learn, your code easier to understand and maintain, and libraries more composable. Many parts of the Aria library adopt `mixin`s to bring code reuse, and you can too.

```aria
mixin Double {
    func double() {
        return this + this;
    }
}

extension Int {
    include Double
}

extension Float {
    include Double
}

func main() {
    println(3.double());
    println(3.14.double());
}
```

Aria has a simple yet usable standard library, with date/time handling, networking, file system access, JSON support and more.

```aria
import Instant from aria.date.instant;

func main() {
    val now = Instant.now();

    println("The current date and time is: {0}".format(now));
}
```

```aria
import JsonValue from aria.json.parser;

func main() {
    val json_data = JsonValue.parse('{"name": "Aria", "version": "0.9"}')!.flatten();
    println("Language: {0}, Version: {1}".format(json_data["name"], json_data["version"]));
}
```

Aria is currently supported on Linux and macOS. Contributions for other operating systems are welcome and encouraged!

## A Taste of Aria

Aria is easy to learn. Here's a [quick example](https://github.com/arialang/aria/blob/master/examples/github_user.aria) that fetches data from a web API and prints the result.

In this example, Aria fetches user data from GitHub’s API and prints the number of public repositories for a given user. This shows how simple it is to interact with external APIs and handle dynamic data in Aria.

```aria
# github_user.aria
import Request from aria.network.request;
import JsonValue from aria.json.parser;

val whoami = "egranata";

func main() {
    val request = Request.new("https://api.github.com/users/{0}".format(whoami));
    request.headers["User-Agent"] = "AriaLang/1.0";
    val response = request.get();

    if response.status_code == 200 {
        val user_data = JsonValue.parse(response.content)!.flatten();
        println("User {1} has {0} public repositories.".format(user_data["public_repos"], whoami));
    } else {
        println("Failed to fetch user data. Status: {0}".format(response.status_code));
    }
}
```

Running this is as simple as:
```shell
$ aria github_user.aria
User egranata has 5 public repositories.
```

## Getting Started

Ready to try Aria? Want to contribute to the language? Great! Whether you’re fixing bugs, adding features, or improving the documentation, we’d love your help!

If it's your first time contributing, check out [good starter bugs](https://github.com/arialang/aria/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22) or [help wanted](https://github.com/arialang/aria/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22help%20wanted%22) on GitHub.

For all this and more, visit [our website](https://arialang.github.io/)!
