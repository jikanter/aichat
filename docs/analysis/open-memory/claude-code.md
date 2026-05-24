# Claude Code Memory Research

<Discovery>
- Claude Code holds its project memory in `~/.claude/projects/<path>/`, where path is a dash seperated absolute path of the project 
    directory
</Discovery>

## Directory contents
<Discovery>
    The directory contains a set of jsonl files (that appear like traces) of each turn per project,
    as well as optional additional project context.

    One such kind of context is a folder "memory", which contains 
    markdown-with-yaml frontmatter that aichat might want to interoperate with
</Discovery>
