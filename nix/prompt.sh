#!/usr/bin/env bash

project_name="$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")"
project_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

__git_prompt() {
    local rel branch marks line

    rel="$(realpath --relative-to="$project_root" "$PWD" 2>/dev/null)"

    branch=""
    marks=""

    while IFS= read -r line; do
        case "$line" in
            "# branch.head "*)
                branch="${line#"# branch.head "}"
                ;;
            "# branch.ab "*)
                [[ "$line" =~ \+([0-9]+) ]] &&
                    (( ${BASH_REMATCH[1]} > 0 )) &&
                    marks+="↑"

                [[ "$line" =~ -([0-9]+) ]] &&
                    (( ${BASH_REMATCH[1]} > 0 )) &&
                    marks+="↓"
                ;;
            "?"*)
                [[ "$marks" == *"+"* ]] || marks+="+"
                ;;
            "1 "*|"2 "*|"u "*)
                [[ "$marks" == *"*"* ]] || marks+="*"
                ;;
        esac
    done < <(git status --porcelain=v2 --branch 2>/dev/null)

    if [[ -n "$branch" ]]; then
        PS1="🦀 $project_name:$rel [$branch$marks] \$ "
    else
        PS1="🦀 $project_name:$rel \$ "
    fi
}

PROMPT_COMMAND=__git_prompt
