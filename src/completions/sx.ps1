# PowerShell completion for sx. Load it with:  sx completions powershell | Out-String | Invoke-Expression
Register-ArgumentCompleter -Native -CommandName sx -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $words = @($commandAst.CommandElements |
        Where-Object { $_.Extent.EndOffset -le $cursorPosition } |
        Select-Object -Skip 1 |
        ForEach-Object { $_.ToString() })
    $arguments = @('__complete')
    if ($wordToComplete -eq '') { $arguments += '--new' }
    $arguments += $words
    & sx @arguments | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}
