import re, sys
# Pull a named step's `run:` body out of a workflow, verbatim. Reading
# the file beats retyping it: a retyped step is a different step, which
# is how `cut -d'\"'` reached four runners.
path, name = sys.argv[1], sys.argv[2]
lines = open(path).read().split('\n')
i = next(n for n, l in enumerate(lines) if l.strip() == f'- name: {name}')
while 'run:' not in lines[i]:
    i += 1
run = lines[i].split('run:', 1)[1].strip()
if run in ('|', '>'):
    body, indent = [], None
    i += 1
    while i < len(lines):
        l = lines[i]
        if l.strip() == '':
            body.append('')
        else:
            cur = len(l) - len(l.lstrip())
            if indent is None:
                indent = cur
            if cur < indent:
                break
            body.append(l[indent:])
        i += 1
    run = '\n'.join(body).rstrip() + '\n'
sys.stdout.write(run if run.endswith('\n') else run + '\n')
