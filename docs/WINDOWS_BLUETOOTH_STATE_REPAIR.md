# Windows BLE 持久安全状态：接口边界与定点修复设计

调查日期：2026-10-01。本文件区分已验证事实、接口契约和待执行实验。
包证据见 [安全状态调查](WINDOWS_BLUETOOTH_SECURITY.md)，公开案例见
[恢复方案调查](WINDOWS_BLUETOOTH_WORKAROUNDS.md)。

## 本轮新增验证

1. 15:22:51（UTC+08:00）对已确认的控制器 AssociationEndpoint 执行一次
   `pairtool /disassociate`。退出码为 0，标准输出和标准错误均为空。
2. 操作前后在内存比较目标 BTHPORT 条目：13 个值、owner/group/DACL 和最后写入时间均未变化。
   没有输出密钥内容或其散列。该命令在本次状态下没有清理持久安全信息。
3. 独立进程读取设备信息及新建 AssociationEndpoint，均为
   `IsPaired=false`、`CanPair=false`、`ProtectionLevel=Encryption`。
4. 普通令牌启用备份／恢复特权失败，错误为 `ERROR_NOT_ALL_ASSIGNED`（1300）。
   UAC 提升后的只读预检成功启用 `SeBackupPrivilege`、`SeRestorePrivilege`，
   并以 `REG_OPTION_BACKUP_RESTORE` 打开同一个现有条目；返回 `REG_OPENED_EXISTING_KEY`。
   预检后值、权限和最后写入时间仍相同。没有试写、删除、修改 ACL 或重载适配器。
5. 已保存并重新读取当前用户范围的 DPAPI 加密备份。解密内容与原快照完全相同，
   包括全部有类型的值及 owner/group/DACL。没有保存 SACL，没有尝试真实目标还原。
6. 独立 HKCU 临时条目使用合成数据测试七种注册表类型：二进制、DWORD、QWORD、
   字符串、可展开字符串、多字符串和 REG_NONE。中途删除后的还原、空条目还原、
   删除条目后携带原安全描述符重建均通过；值、类型及 owner/group/DACL 一致。
   临时条目已删除。没有将真实蓝牙密钥写入测试条目。

这些结果不代表连接已修复。HKCU 合成测试也不证明受保护 HKLM 目标的在线还原已经通过。
原始捕获、备份、预检和测试输出保留在 Git 忽略的本地诊断目录。

本地已准备固定目标的 `repair_target_security.py` 原型。默认运行只核验备份、目标和物理适配器，
此只读模式已通过；真实操作需要显式 `--apply-confirmed-target` 及提升权限，尚未运行。
原型拒绝已知连接客户端正在运行的情况，在清理中途失败时尝试条件还原，并在所有退出路径
尝试重新启用适配器。其原生类型还原函数也已通过上述七种合成类型测试。
它没有实现重配对后人工回滚或受保护 HKLM 叶子被系统删除后的重建，不能视为完整生产恢复工具。

## 有没有通用的强制清理 API

在已检查的公开 Windows SDK 和 Microsoft 文档中，没有找到这样的受支持契约：
**即使设备关联报告未配对，也保证清空该 BLE 对端的持久密钥及主机运行时安全状态。**
这不是断言系统内部不存在删除密钥的实现。

| 层次或入口 | 已核实用途 | 当前边界 |
| --- | --- | --- |
| WinRT `DeviceInformationPairing.UnpairAsync()` | 取消设备关联 | 本机 SDK 只有无参数入口，没有强制清密钥选项；此前返回 AlreadyUnpaired，不能据此认定安全库为空 |
| PairTool `/disassociate <endpoint>` | 独立工具取消关联 | 本轮返回 0，但目标持久安全信息没有变化；文档没有强制清理开关 |
| `deviceassociation.dll` 的 `DafStartRemoveAssociation` | 本机 DLL 导出且 PairTool 导入的内部入口 | 未找到受支持的公开 ABI 或强制参数；直接调用它并不自动获得比 PairTool 更强的行为 |
| `BluetoothRemoveDevice` | 删除 remembered device 的相关信息 | 不能把经典 API 的文档推广为对所有未关联 BLE 条目的强制清理保证 |
| 公开 `bthioctl.h` | 设备信息、断开、供应商命令、SDP 等 | 未找到上述 BLE 安全状态强制清理契约；供应商 HCI 入口不是通用安全库管理 API |
| 内核 `BRB_STORED_LINK_KEY` | 名称出现在 SDK 枚举中 | Microsoft 标为内部使用；没有公开可用的负载契约，不作为本项目恢复接口 |
| `INDICATION_UNPAIR_DEVICE` | 注册回调时选择取消配对通知 | 它是通知选择位，不是发起取消配对的命令 |
| PnP 重启／禁用／启用 | 控制指定设备实例的生命周期 | 可以重载适配器，但不保证清理持久密钥 |
| 注册表备份／恢复及值删除 API | 管理有权限访问的注册表条目 | API 公开，但 BTHPORT 密钥存储格式及其与运行时缓存的同步不是受支持的蓝牙修复契约 |

依据：[UnpairAsync](https://learn.microsoft.com/en-us/uwp/api/windows.devices.enumeration.deviceinformationpairing.unpairasync?view=winrt-26100)、
[PairTool 语法](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pairtool-command-syntax)、
[BluetoothRemoveDevice](https://learn.microsoft.com/en-us/windows/win32/api/bluetoothapis/nf-bluetoothapis-bluetoothremovedevice)、
[BRB_TYPE](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/bthddi/ne-bthddi-_brb_type)、
[取消配对通知](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/bthddi/ns-bthddi-_brb_l2ca_register_server)。
内部导入／导出结论来自本机 PE 表检查，不是根据函数名推断调用参数或完整调用链。

底层 HCI 的 `Delete Stored Link Key` 也不能当作这个 BLE 问题的通用修复：
[Bluetooth SIG 的 HCI 规范，第 7.3.10 节](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/host-controller-interface/host-controller-interface-functional-specification.html)
将其限定为删除 BR/EDR Controller 保存的 Link Key。它没有提供清空 Windows 主机 BLE LTK
持久存储的契约。不能因为命令名称包含“删除密钥”就通过供应商入口发送它来解决本机问题。

## 权限路径：不必先接管权限或建立 SYSTEM 服务

Microsoft 的 [RegCreateKeyExW 文档](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regcreatekeyexw)
说明，`REG_OPTION_BACKUP_RESTORE` 会按已启用特权取得访问权：
备份特权提供读取权限，恢复特权在当前 Windows 上提供写入及 DELETE 权限。
它不会给一个不含相应特权的令牌补充特权；必须检查
[AdjustTokenPrivileges](https://learn.microsoft.com/en-us/windows/win32/secauthz/enabling-and-disabling-privileges-in-c--)
后的 `ERROR_NOT_ALL_ASSIGNED`，并在操作结束时恢复原特权状态。

只读预检先正常打开**已存在的精确目标叶子**，再以这个句柄和空子键名取得特权句柄。
没有向 `RegCreateKeyExW` 传入可能补建整段路径的目标字符串。
该方式避免在权限探测过程中意外创建不存在的设备目录；不存在的目标应立即终止。

生产实现可扩展现有 [管理员客户端](../src/admin_client.rs)、
[认证管道](../src/admin_ipc.rs)和 [单次管理员执行器](../src/admin_worker.rs)。
应新增明确的命令与结果类型，而不是开放“任意注册表路径／任意命令”入口。
当前执行器仍只实现重启 `bthserv`；本轮没有把真实密钥清理接入应用。

## 下一轮定点修复实验

本实验会改变系统内部安全记录，并短暂中断当前物理适配器上的其他蓝牙连接。
它应由用户发起一次，保留阶段结果，不作为后台自动重试。

1. 核实当前适配器地址、物理 PnP 实例及控制器地址。暂停目标自动连接，关闭目标 GATT 会话、
   通知及句柄。记录目标安全快照、新建关联信息及 UTC 时间。
2. 验证当前用户 DPAPI 备份可解密、类型完整、目标身份一致。备份只保存在本地。
   更换管理员账号时不能假设仍可解密；本次预检在原用户提升令牌下完成。
3. 记录适配器初始启用状态，禁用已确认的物理实例，并查询实际 PnP 状态。
   不能只以工具退出码认定驱动已停止；需要重启或禁用失败时不修改密钥。
4. 停止后再次读取精确目标条目。若值或权限与备份不同，则退出并重新备份，不能用旧快照继续。
5. 第一轮只清除**该目标叶子的全部 13 个值**，保留叶子及其原权限。
   不仅删除 LTK 而留下其余绑定信息，也不修改整个 Keys、适配器目录或 Enum 树。
   用受限特权句柄逐项删除、刷新并确认没有值及子键。
   空叶子能否被本机堆栈正确视为无安全记录，是本轮需要验证的假设。
6. 所有退出路径均尝试恢复适配器初始状态，并记录真实启用结果。
   清理中途失败时，在适配器仍停止且条目没有外部修改的条件下恢复基线；失败应单独报告。
7. 重载后先读取安全条目，不立即发起 GATT 连接。若原值自行回来，保存阶段证据，停止清理循环。
   这表明还存在缓存写回或其他状态来源，需要定位写入者。
8. 控制器进入 RGB 配对模式后，使用新建 AssociationEndpoint 独立配对。
   配对结束后再创建保持连接的 GATT 会话，读取未缓存服务并接收有效输入。
9. 正常退出，新进程重连至少两次，再验证控制器睡眠唤醒及断电恢复。
   包含完整 SMP 的新捕获用于区分新协商和旧安全输入复用。

适配器操作依据 [PnPUtil](https://learn.microsoft.com/en-us/windows-hardware/drivers/devtest/pnputil-command-syntax)；
值删除依据 [RegDeleteValueW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regdeletevaluew)。
公开 API 可执行这些动作，不等于 Microsoft 保证这种内部存储修复有效。

回滚不能覆盖重配对后新生成的安全信息。已出现新值时，应保留并分析，而不是自动恢复旧失效密钥。
若系统删除空叶子，重建需要验证父条目仍存在，并使用备份的安全描述符；
不能默默继承新权限。合成测试验证了这种重建的序列化与权限保留，但真实保护目标尚未验证。
备份不包含 SACL，因此不能声称能够完整还原审计设置。恢复旧密钥只能恢复实验前状态，不代表修复连接。

## 怎样判定实验结果

| 观察 | 能得出的结论 | 后续动作 |
| --- | --- | --- |
| 重载后没有原安全值，首次连接及多次新进程重连通过 | 支持定点清理加重载可修复本机状态 | 再完成睡眠、断电验证，之后才考虑产品化 |
| 原安全值在配对前自动返回 | 清理没有跨越所有状态层，或存在写回 | 用短时注册表事件追踪定位写入进程及调用栈，不能反复盲删 |
| 旧值未返回，仍没有 SMP Response | 排除了本轮原持久值直接复用，未排除其他主机或外设问题 | 对齐新 HCI、SMP、WPR 时序，比较另一主机或适配器 |
| 新配对成功、首次有效输入成功，但下一进程重连失败 | 仍没有持久修复 | 对照新生成密钥、安全输入及控制器是否保留绑定 |
| 只有 PairTool 退出 0 或 PairAsync 返回成功 | 工具或 API 已完成，不证明数据链路可用 | 继续验证有效输入及独立重连 |

需要定位写回时，可使用 [Process Monitor](https://learn.microsoft.com/en-us/sysinternals/downloads/procmon)
的注册表事件及调用栈，对**精确目标路径**做短时捕获，并与 HCI 时间对齐。
追踪文件可能包含密钥，应留在本地；公共报告只输出操作、时间、进程、状态与比较结果。
Windows WPR 配置见安全状态调查。没有匹配的内部格式或符号时，不能承诺解析所有 WPP 原因。

## 应用实现职责

恢复协调器只负责阶段、超时、取消和结果组合。关联操作、加密备份、特权注册表操作、
适配器控制及连接验证应各自独立。各阶段至少记录 `operation_id`、目标、阶段、耗时、
状态码、前后关联字段、安全条目是否存在及是否改变；不记录密钥或密钥散列。

管理员管道只传限定目标和阶段结果。现有消息上限为 4096 字节，不应顺手扩成任意大小，
也不应把完整解密备份塞进普通日志或错误文本。管理员执行器应在原用户提升令牌下
完成 DPAPI 解密及受限操作；取消和失败同样要恢复适配器并报告还原结果。

`IsPaired=false` 只说明当前关联读数。普通日志应表述为“Windows 未报告设备关联”，
而非“没有密钥”“没有加密”。GATT `Unreachable` 也不能单独触发安全记录清理。
只有验证通过的恢复步骤，才能提供给日常使用流程。
