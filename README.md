# Solana 地址生成器

本工具可以用来生成以特定前缀开头的 Solana 钱包地址，靓号搞起来

## 使用示例

查找以 "88888" 或 "6666" 或 “Retard” 开头的地址，额外保存 5 个非匹配地址，使用 8 个线程：

./solana-vanity-address -p 88888,6666,Retard -n 5 -t 8

### MacOS 用户： 1.打开终端 2.切换到程序所在目录 3.运行命令:（二选一即可）

./solana-vanity-address -p 88888,6666 -n 10

### Windows 用户： 1.打开命令提示符或 PowerShell 2.切换到程序所在目录 3.运行命令:

solana-vanity-address.exe -p 88888,6666 -n 10

## 命令行参数：

-p, --prefixes <PREFIXES> 地址前缀，多个前缀用逗号分隔
-n, --non-matching-count <NON_MATCHING_COUNT>
要保存的非匹配地址的数量 [默认值: 0]
-t, --threads <THREADS> 线程数量，0 表示使用所有可用线程 [默认值: 0]
-o, --output <OUTPUT> 非匹配地址的输出文件 [默认值: "data/solana_addresses.csv"]
-m, --matched-output <MATCHED_OUTPUT>
匹配地址的输出文件 [默认值: "data/matched_addresses.csv"]
-h, --help 显示帮助信息
-V, --version 显示版本信息

## 输出文件

- 匹配的地址将保存在 data/matched_addresses.csv 中（可通过 -m 参数修改）
- 非匹配的地址（如果指定了 -n 参数）将保存在 data/solana_addresses.csv 中（可通过 -o 参数修改）
- 输出文件格式为 CSV，包含两列：地址和私钥。

## 注意事项

- 生成以特定前缀开头的地址是一个计算密集型任务，可能需要相当长的时间
- 请安全保管生成的私钥
- 程序会自动创建 data 目录（如果不存在）
