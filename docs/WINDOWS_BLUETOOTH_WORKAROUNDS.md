# Windows BLE 关联与密钥状态不一致：多语言检索与恢复方案

调查日期：2026-10-01。对象为本项目已确认的 Gear VR 控制器。

## 结论

已用中文、英文、日文和德文检索公开问题，并核对 Microsoft 文档及第一手记录。
没有找到能够直接证明已修复本机组合的补丁或通用 API。
最接近的 Microsoft 回复仍建议删除主机关系后重新配对；本机的难点是关联 API
报告未配对，而失效安全信息仍存在，普通取消配对没有报告实际删除。

本轮新增只读检查确认：

- 新建 AssociationEndpoint 再次报告 `IsPaired=false`、`CanPair=false`、`ProtectionLevel=Encryption`。
- 当前适配器对应的目标 BTHPORT 密钥条目仍存在，包含 `LTK`、`ERand`、`EDIV` 等字段。
- 本地内存比较确认，这三个值与完整捕获中四次加密命令的输入一致。
  LTK 按原始字节相等；ERand 按小端整数相等；EDIV 相等。
- 条目最后写入时间为 10:52:06.544（UTC+08:00），与首次恢复时的配对成功事件相邻。
  这不是创建时间，不能单独证明密钥是那时新生成的。
- 本轮没有修改密钥、PnP 节点、适配器或服务。只保存字段名称、数据类型和比较布尔值。

所以，已经可以确认**未关联读数与持久密钥条目并存**，而不是只假设有不可见内存缓存。
这不证明历史命令具体从哪个存储层取值，也不证明控制器侧为何无法提供匹配密钥。
详细包时序见 [安全状态调查](WINDOWS_BLUETOOTH_SECURITY.md)。

## 检索范围与公开案例

代表检索词包括：

| 语言 | 检索词 |
| --- | --- |
| 中文 | Windows BLE 未配对、旧密钥、密钥丢失、删除配对、单端配对 SMP |
| 英文 | IsPaired false LTK、AlreadyUnpaired、stale keys、Key Missing re-pair |
| 日文 | ペアリング情報、削除済み、古い情報、鍵、再接続、ゴーストデバイス |
| 德文 | nicht gekoppelt、alte Kopplungsinformationen、Schlüssel löschen、Gear VR |

检索返回本项目自身时不作为独立证据。相同 Microsoft 问题的不同语言页面、
同一提问者的跨站帖子也不算独立复现。一般蓝牙教程没有底层记录时不能用于确认本机根因。

### 英文：单端删除绑定后 Windows 仍加密并断开

[2024 年 Stack Overflow 原始问题及 Microsoft 回复](https://stackoverflow.com/questions/78757568/how-to-establish-ble-connection-without-going-through-smp-process-in-single-ende)
报告 Android 一侧删除绑定后，Windows 继续加密、发生 Key Missing 并主动断开。
提问者尝试了最低 GATT 保护级别、MaintainConnection，以及不同缓存和服务查询路径，未绕过故障。
Microsoft 回复称，当时没有 API 告知应用手机侧已经删除配对，建议从 Windows 侧移除关系后重新配对。
该回答没有验证 `AlreadyUnpaired` 却保留同一安全输入的情况，不能作为本机恢复已成功的证据。

[2024 年 Microsoft Q&A 的绑定密钥丢失问题](https://learn.microsoft.com/en-us/answers/questions/1609887/bluetooth-le-programmatically-determine-if-a-perip)
另记录应用只收到 `Unreachable`，没有特定密钥缺失结果。不能据此把所有不可达错误自动判为 `0x06`。

[同型号 Gear VR 的公开 issue](https://github.com/minhe7735/GearVR-Controller-WIndows/issues/13)
报告多个实现都在通知订阅时出现 `Unreachable`，配对调用返回 AlreadyPaired。
没有 HCI 密钥证据和已验证修复；它只说明同型号曾出现相似高层症状。

### 中文：直接写入密钥不等于建立设备关联

[ZMK 双系统现场记录，2026-08-27](https://www.deletoe.com/2026/08/27/%E5%8F%8C%E7%B3%BB%E7%BB%9F%E4%B8%8B%EF%BC%8Czmk-%E8%93%9D%E7%89%99%E9%94%AE%E7%9B%98%E4%B8%BA%E4%BB%80%E4%B9%88%E6%8D%A2%E6%A7%BD%E4%BD%8D%E4%B9%9F%E8%BF%9E%E4%B8%8D%E4%B8%8A/)
作者报告，向 Windows 导入另一系统的 LTK 等信息后，仍出现 IsPaired=False、GATT Unreachable 和 PairAsync Failed；
最终采用 Windows 先配对、再向 Linux 同步绑定的方向。这里是有意导入密钥及双系统身份冲突，
不是本机清理失败的独立复现。它支持把密钥库和设备关联分别验证，不能套用键盘专用清除命令。

[Microsoft 中文 Q&A，2026 年](https://learn.microsoft.com/zh-cn/answers/questions/5822074/win11)
有人报告删除注册表和设备节点后记录仍在，另一用户报告删除全部蓝牙设备、重启并重装驱动后恢复。
后续回复也有更换主板后仍失败的情况。该组恢复同时改变多个变量，没有确认 LTK 根因；
不能把批量删除或重装所有设备作为本项目默认方案。

### 日文：目标信息残留，重新建关系后恢复的个案

[ZMK ゴーストデバイス 原始现场记录](https://www.naenote.net/entry/zmk-bluetooth-lag-windows-ghost-device)
作者报告系统设置无法删除、PnP 状态 Unknown、DevCon 提示设备不存在；
最终手动删除 Enum/BTHLE 目标信息、重启并清空键盘端绑定后恢复。
它没有比较 HCI 的 LTK、Rand、EDIV，也同时改变了主机和外设状态。
可用于提出“残留枚举信息参与故障”的假设，不能证明本机需要手改 Enum 或其权限。
本机当前目标节点为 Status=OK，而不是该文的 Unknown。

### 德文：Gear VR 的重复 IRK 是另一种机制

[两个 Gear VR 分发相同 IRK 的 Microsoft Q&A](https://learn.microsoft.com/de-de/answers/questions/4184862/bluetooth-le-ger-te-mit-gleichen-identity-resolvin)
报告第二台控制器因身份解析密钥重复被拒绝。IRK 管身份解析，LTK 管链路加密。
本机从 2026-09-27 起查询 BTHUSB 35 未发现记录；本轮新查也未发现事件 22 的调试密钥拒绝记录。
这两个错误均没有成为当前故障的证据。不能因同型号就套用修改 IRK 或启用调试密钥的绕过。

## 可测试的恢复路径

以下为候选实验，尚未通过稳定重连验证。每轮保存基线，并且只改变一个变量。

| 顺序 | 操作 | 能解决或区分什么 | 成功判断与限制 |
| --- | --- | --- | --- |
| 1 | 精确目标 AssociationEndpoint 的 PairTool `/disassociate` | 用独立系统工具对照已返回 AlreadyUnpaired 的 WinRT 路径 | 核对工具结果、目标关系及密钥条目；退出码 0 不能替代实际状态。没有找到文档保证它能强制清除未关联的密钥 |
| 2 | 重启当前物理蓝牙适配器 | 比较驱动/主机运行时状态；可能免去整机重启 | 重启后核对密钥条目与首次加密输入。如果仍加载相同失效密钥，单纯重启不是修复 |
| 3 | 精确目标的 PnP 节点重新枚举 | 对照服务/枚举残留是否影响关联与恢复 | 只处理已核实目标和其子节点；删除 PnP 节点不等于删除绑定密钥 |
| 4 | 备份后，仅清理已确认失效的目标持久密钥条目，并重载适配器 | 对照“无关联但持久安全信息仍被使用”的状态 | 高级恢复实验，存储格式没有受支持的强制清空 API 保证；必须有可验证备份与恢复步骤。不能自动删除整个 Keys 或适配器目录 |
| 5 | 另一适配器或稳定版 Windows 主机 | 区分当前适配器身份、驱动和 Insider 构建的影响 | 分别改变变量；另一适配器会改变身份和驱动，并非只改变一个缓存。先借用现有设备，不凭搜索结果要求购买 |
| 6 | 外部 BLE 主机转 USB/串口/网络，或独立 Linux BlueZ 后端 | 真正绕开 Windows BLE 安全状态机 | 需要新增传输实现、认证与部署；更换同样基于 WinRT 的语言库并没有绕开系统堆栈 |

第 1 项依据 [Microsoft PairTool](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pairtool-examples)，
仅按端点 ID 操作；没有文档支持向 `/disassociate` 传容器 ID 来强行删除全部安全信息。
第 2、3 项依据 [PnPUtil](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pnputil-command-syntax)。
PnP 重启、射频开关、bthserv 重启和底层 ACPI 复位是不同操作。
[Microsoft 无线电恢复文档](https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/bluetooth-radio-error-recovery)
描述的是依赖驱动与 ACPI 的恢复机制，不能把任意 PnP 重启等同于这些复位或密钥清空。

本机已核实的物理适配器实例可用于下一轮管理员终端对照：

```powershell
# 实验前先确认此实例仍是当前 Qualcomm 适配器，并关闭目标 GATT 会话、开始新的一轮短时捕获。
Get-PnpDevice -InstanceId 'USB\VID_10AB&PID_9308\6&8B9A1DC&0&2'
pnputil.exe /restart-device 'USB\VID_10AB&PID_9308\6&8B9A1DC&0&2'
# 不使用 /reboot；恢复期间该适配器上的其他蓝牙连接会中断。
```

此重启命令本轮未执行。恢复后先核实射频和目标广播，再保持 RGB 模式完成独立配对，
配对结束后才创建 GATT 会话。第一轮真实收到有效输入后，正常关闭、新进程重复至少两次，
再测试控制器睡眠及断电。所有阶段比较完整 SMP 与安全输入，不公开密钥。

持久条目已经确认存在，所以适配器重启不是最终解答：如果条目仍被重新加载，
应转向第 1、3 项的受支持定点清理对照；第 4 项需要独立的备份、回滚与清理验证设计。

## 不能当作现成修复的办法

- `ConfirmOnly + None`：来自 [Microsoft FAQ](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/bluetooth-dev-faq)，
  但本机三次自定义配对与独立 PairTool 尝试均未恢复。`None` 是最低要求，不保证关闭已存在的链路安全状态。
- 换 Cached/Uncached、降低 GATT 保护要求、增加通知重试：不能删除持久安全信息；本机失败早于协议初始化。
- `BluetoothRemoveDevice`：其 [文档](https://learn.microsoft.com/en-us/windows/win32/api/bluetoothapis/nf-bluetoothapis-bluetoothremovedevice)
  要求设备是 remembered device，不能据此当作对所有 BLE 失效密钥的强制删除接口。
- 仅重启 bthserv：可能影响服务状态，没有证据保证清空 BTHPORT 的目标持久密钥或驱动缓存。
- 随机更换 LTK/IRK、启用调试密钥、降低系统安全策略：不建立双方相同的有效绑定，不适合作为恢复机制。

## 项目应怎样实现

当前 [管理员恢复实现](../src/admin_worker.rs)只重启 `bthserv`，不包含适配器重启或定点密钥清理。
不能把“服务已重新启动”显示为“设备连接已修复”。

建议将恢复做成串行、有界的流程：

1. 连接协调器关闭目标通知、会话和句柄；暂停该目标的自动重连。
2. 诊断组件记录新建终结点的三个关联字段、阶段结果，以及可用的底层故障证据。
3. 恢复组件分别实现目标取消关联、目标枚举恢复、明确选定适配器重启；每项保留独立结果。
   需要权限的操作通过现有单次、认证的管理员管道执行，限制命令和目标身份。
4. 配对组件使用新建 AssociationEndpoint；PairAsync 完成前不创建保持连接的 GATT 会话。
5. 验证组件读取未缓存服务并等待有效控制器输入，正常断开后验证新进程重连。
   只有这一步通过，才报告恢复成功。

这些职责应分别测试和组合；没有观察到过程改变前，不给所有 Unreachable 加自动取消配对。
普通连接日志也应只描述观察：`IsPaired=false` 应写“Windows 未报告设备关联，开始 GATT 连接”，
不应声称“没有配对”或“没有加密”；`IsPaired=true` 也不证明既有密钥可用。

真正跨 Windows 蓝牙堆栈的后备方案应实现独立 Transport，复用现有协议解码与输入映射。
[BlueZ Management 协议](https://github.com/bluez/bluez/wiki/MGMT)明确提供清除绑定密钥的管理语义；
但它属于 Linux，需要独立后端或网关，不能直接拿来修复 Windows WinRT 的内部状态。
