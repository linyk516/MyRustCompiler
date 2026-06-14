# MyRustCompiler

同济大学 2026 春《编译原理》课程作业。项目实现了一个类 Rust 语言编译器，支持从源程序到词法分析、语法分析、语义分析、借用检查、LLVM-like IR 生成，以及通过 `clang` 生成可执行文件。

本文档主要说明如何构建、运行和查看编译结果。

## 环境要求

基础使用：

- Rust toolchain，建议使用当前稳定版或项目所用的 edition 2024 可用版本
- Cargo

生成可执行文件或运行后端测试：

- LLVM/clang
- macOS Homebrew 环境下会自动尝试查找：
  - `clang`
  - `/opt/homebrew/bin/clang`
  - `/opt/homebrew/opt/llvm/bin/clang`

使用 GUI：

- Node.js
- npm
- Tauri 2.x 所需的系统依赖

## 快速开始

编译并运行一个示例：

```bash
cargo run -- example_source/source1.txt
```

显示更多阶段输出：

```bash
cargo run -- example_source/source1.txt \
  --show-tokens \
  --show-ast \
  --show-hir \
  --show-typecheck \
  --show-thir \
  --show-ir
```

生成 LLVM IR 文件：

```bash
cargo run -- example_source/extern/printf.txt -ll
```

这会在源文件旁边生成同名 `.ll` 文件，例如：

```text
example_source/extern/printf.ll
```

直接生成可执行文件：

```bash
cargo run -- example_source/extern/printf.txt -o /tmp/myrust_printf
/tmp/myrust_printf
```

## CLI 用法

基本形式：

```bash
cargo run -- <source-file> [options]
```

当前 CLI 接受多个文件路径参数，但编译流程默认只处理第一个输入文件。

常用选项：

| 选项 | 作用 |
| --- | --- |
| `-r`, `--rebuild` | 重新构建前端缓存 |
| `-v`, `--verbose` | 输出更详细的编译信息 |
| `--show-tokens` | 显示词法分析结果 |
| `--show-ast` | 显示 AST |
| `--show-hir` | 显示 HIR |
| `--show-typecheck`, `--show-typeck` | 显示类型检查结果 |
| `--show-thir` | 显示 THIR |
| `--show-ir` | 显示 LLVM-like IR |
| `-ll`, `--ll` | 输出 `.ll` 文件 |
| `-o <OUTPUT>` | 调用 `clang` 生成目标可执行文件 |
| `--color` | 强制启用彩色诊断 |
| `--no-color` | 禁用彩色诊断 |

示例：

```bash
cargo run -- example_source/test2.txt --show-typecheck --show-ir
```

## 生成与运行

### 只生成 LLVM IR

```bash
cargo run -- example_source/extern/printf.txt -ll
```

如果 IR 生成成功，会输出：

```text
LLVM IR written to example_source/extern/printf.ll
```

之后可以手动使用 `clang` 编译：

```bash
clang example_source/extern/printf.ll -o /tmp/myrust_printf
/tmp/myrust_printf
```

### 直接生成可执行文件

```bash
cargo run -- example_source/extern/printf.txt -o /tmp/myrust_printf
/tmp/myrust_printf
```

`-o` 会在编译成功后自动生成临时 `.ll` 文件，并调用 `clang` 完成链接。外部函数如 `printf`、`scanf` 由系统 C 标准库负责链接。

### 作为库与 C 接口调用

如果源程序没有 `main`，更适合作为库输出 `.ll`，再由 C 程序一起链接。可以参考脚本：

```bash
bash scripts/test_c_interface.sh
```

该脚本会生成类 Rust 函数的 LLVM IR，再用 C 程序调用这些函数。

## GUI 用法

GUI 使用 Tauri + Vite + React 实现，默认构建不包含 GUI 依赖。首次使用前安装前端依赖：

```bash
npm --prefix gui install
```

启动 GUI：

```bash
cargo run --features gui --bin myrust-gui
```

GUI 提供：

- 源码编辑器
- 编译按钮
- 构建并运行按钮
- stdin 输入区
- Diagnostics、Tokens、CST、AST、HIR、Typecheck、THIR、IR 等结果页
- 内置示例加载

也可以单独启动前端开发服务器：

```bash
npm --prefix gui run dev
```

## 示例程序

常用示例位于 `example_source/`：

| 路径 | 说明 |
| --- | --- |
| `example_source/source1.txt` | 综合语法与控制流示例 |
| `example_source/test2.txt` | 引用和解引用示例 |
| `example_source/extern/printf.txt` | 外部函数和字符串示例 |
| `example_source/hanoi/hanoi.txt` | 使用 `printf`、`scanf` 的汉诺塔示例 |
| `example_source/struct_patterns.txt` | struct 与 let pattern 示例 |
| `example_source/type/` | 类型检查正例和反例 |

运行 printf 示例：

```bash
bash scripts/test_printf.sh
```

运行 `-o` 后端输出示例：

```bash
bash scripts/test_output_executable.sh
```

运行汉诺塔示例：

```bash
cargo run -- example_source/hanoi/hanoi.txt -o /tmp/myrust_hanoi
printf "3\n" | /tmp/myrust_hanoi
```

## 当前语言能力

当前类 Rust 语言主要支持：

- 函数定义和函数调用
- `extern fn` 外部函数声明
- `printf`、`scanf` 这类 C 风格外部函数调用
- `i32` 以及已扩展的基础整数和布尔类型
- `str` 字符串字面量，按 C 字符串指针传递
- `let`、`mut`、赋值、返回
- `if`、`while`、`loop`、`for`
- 数组、元组、字段访问和下标访问
- 引用、可变引用、解引用
- struct 和 let pattern
- 基础类型检查、确定赋值检查、部分借用规则检查
- LLVM-like IR 输出和 `clang` 后端链接

## 测试

运行 Rust 测试：

```bash
cargo test
```

检查默认 CLI 构建：

```bash
cargo check
```

检查 GUI 构建：

```bash
cargo check --features gui --bin myrust-gui
```

运行前端测试：

```bash
npm --prefix gui test
```

构建前端静态资源：

```bash
npm --prefix gui run build
```

## 常见问题

### 找不到 clang

使用 `-o` 或运行后端脚本时需要 `clang`。请安装 LLVM/clang，并确保 `clang` 位于 `PATH` 中。

### `-ll` 和 `-o` 的区别

`-ll` 只输出 LLVM IR 文件，不生成可执行文件。

`-o <OUTPUT>` 会调用 `clang` 生成可执行文件。

### GUI 白屏

先确认前端依赖已安装：

```bash
npm --prefix gui install
```

然后使用：

```bash
cargo run --features gui --bin myrust-gui
```

如果仍然为空白，可以单独启动前端服务检查：

```bash
npm --prefix gui run dev
```
