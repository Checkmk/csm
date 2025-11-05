set -g _CSM_SHELL fish

function csm
    set -l csm_bin (type -P csm)
    if test (count $argv) -ge 2; and test $argv[1] = "env"; and test $argv[2] = "activate" -o $argv[2] = "deactivate"
        eval ($csm_bin $argv)
    else
        $csm_bin $argv
    end
end