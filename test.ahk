#Requires AutoHotkey v2.0
#SingleInstance Force
SetWorkingDir A_ScriptDir

; 添加一个测试热键
F4:: {
    MsgBox("脚本正在运行！")
}

; 显示窗口信息
F1:: {
    try {
        MouseGetPos(,, &WinID)
        WinTitle := WinGetTitle("ahk_id " WinID)
        WinClass := WinGetClass("ahk_id " WinID)
        WinProcess := WinGetProcessName("ahk_id " WinID)
        WinStyle := WinGetStyle("ahk_id " WinID)
        WinExStyle := WinGetExStyle("ahk_id " WinID)
        
        MsgBox(
            "窗口标题: " WinTitle "`n" 
            "窗口类名: " WinClass "`n"
            "进程名: " WinProcess "`n"
            "窗口样式: " WinStyle "`n"
            "扩展样式: " WinExStyle "`n"
            "窗口句柄: " WinID
        )
    } catch as err {
        MsgBox("发生错误: " err.Message)
    }
}

; 显示窗口层级
F2:: {
    MouseGetPos(,, &WinID)
    WinList := WinGetList("ahk_id " WinID)
    
    for index, hwnd in WinList {
        WinTitle := WinGetTitle("ahk_id " hwnd)
        WinClass := WinGetClass("ahk_id " hwnd)
        WinProcess := WinGetProcessName("ahk_id " hwnd)
        
        MsgBox(
            "层级 " index ":`n"
            "标题: " WinTitle "`n"
            "类名: " WinClass "`n"
            "进程: " WinProcess "`n"
            "句柄: " hwnd
        )
    }
}

; 显示窗口消息
F3:: {
    MouseGetPos(,, &WinID)
    WinTitle := WinGetTitle("ahk_id " WinID)
    WinClass := WinGetClass("ahk_id " WinID)
    WinProcess := WinGetProcessName("ahk_id " WinID)
    
    ; 获取窗口的父窗口
    ParentID := WinGetID("ahk_id " WinID)
    ParentTitle := WinGetTitle("ahk_id " ParentID)
    
    MsgBox(
        "当前窗口:`n"
        "标题: " WinTitle "`n"
        "类名: " WinClass "`n"
        "进程: " WinProcess "`n"
        "句柄: " WinID "`n`n"
        "父窗口:`n"
        "标题: " ParentTitle "`n"
        "句柄: " ParentID
    )
} 