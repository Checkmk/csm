$env:_CSM_SHELL = "powershell"

function csm {
    $CSM_BIN = Get-Command csm -CommandType Application | Select-Object -ExpandProperty Source -First 1
    if ($args.Length -ge 2 -and $args[0] -eq "env" -and ($args[1] -eq "activate" -or $args[1] -eq "deactivate")) {
        $output = & $CSM_BIN @args | Out-String
        if ($output.Trim()) {
            Invoke-Expression $output
        }
    } else {
        & $CSM_BIN @args
    }
}