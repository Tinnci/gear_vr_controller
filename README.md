# Gear VR 控制器 Windows 应用程序

这是一个用于 Gear VR 控制器的 Windows 应用程序，允许用户将 Gear VR 控制器作为电脑输入设备使用。

## 功能特点

1. **蓝牙连接管理**
   - 自动搜索并连接 Gear VR 控制器
   - 实时显示连接状态
   - 支持断开连接和重新连接

2. **触摸板控制**
   - 将触摸板映射为鼠标移动
   - 支持触摸板点击作为鼠标点击
   - 高精度的触摸板校准功能
   - 可调节的鼠标灵敏度

3. **按键映射**
   - 支持触发键和返回键的自定义映射
   - 可配置的按键组合

4. **运动控制**
   - 支持陀螺仪数据读取
   - 实时显示传感器数据

5. **用户界面**
   - 现代化的 WinUI 3 界面
   - 实时状态显示
   - 直观的设置界面
   - 校准向导

## 项目结构

```
GearVRController/
├── Models/
│   └── ControllerData.cs          # 控制器数据模型
├── ViewModels/
│   ├── MainViewModel.cs           # 主窗口视图模型
│   └── TouchpadCalibrationViewModel.cs  # 触摸板校准视图模型
├── Views/
│   └── TouchpadCalibrationWindow.xaml   # 触摸板校准窗口
├── Services/
│   └── BluetoothService.cs        # 蓝牙服务
├── Helpers/
│   └── InputSimulator.cs          # 输入模拟器
├── Converters/
│   ├── BoolToControlStateConverter.cs    # 控制状态转换器
│   └── BooleanConverters.cs       # 布尔值转换器
├── MainWindow.xaml                # 主窗口界面
└── App.xaml                       # 应用程序资源
```

## 主要功能模块

### 1. 蓝牙连接 (BluetoothService)
- 处理与 Gear VR 控制器的蓝牙通信
- 实现数据包的解析和处理
- 管理连接生命周期

### 2. 输入模拟 (InputSimulator)
- 模拟鼠标和键盘输入
- 处理触摸板移动到鼠标移动的转换
- 实现按键映射功能

### 3. 触摸板校准 (TouchpadCalibrationViewModel)
- 多步骤校准流程
- 边界检测
- 方向校准
- 自动完成功能

### 4. 主界面控制 (MainViewModel)
- 管理应用程序状态
- 处理用户设置
- 协调各个功能模块

## 使用说明

1. **连接控制器**
   - 启动应用程序
   - 按住控制器触发键进入配对模式
   - 点击"连接"按钮

2. **校准触摸板**
   - 点击"校准"按钮
   - 按照向导完成校准步骤
   - 校准完成后自动保存设置

3. **自定义设置**
   - 调节鼠标灵敏度
   - 配置按键映射
   - 启用/禁用特定功能

## 系统要求

- Windows 10 版本 1809 或更高
- 支持蓝牙 4.0 或更高版本
- WinUI 3 运行时

## 开发环境

- Visual Studio 2022
- .NET 6.0 或更高版本
- Windows App SDK 1.2 或更高版本

## 注意事项

1. 首次使用需要进行触摸板校准
2. 确保控制器电量充足
3. 保持控制器在蓝牙有效范围内
4. 如遇连接问题，请尝试重启控制器

## 未来计划

1. 添加更多自定义选项
2. 优化触摸板响应
3. 支持手势识别
4. 添加配置文件导入/导出功能 