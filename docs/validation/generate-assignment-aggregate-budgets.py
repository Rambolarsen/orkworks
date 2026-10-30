"""Isolated documentation vectors, not source-valid stores or transactions."""
import json

def zero():
    return {r: {s: '0' for s in ('assignment', 'run', 'workspace')} for r in ('records', 'payloads')}

vectors = []
def emit(name, usage, delta, exceeded):
    reason = None if not exceeded else ('assignment-record-capacity-exhausted' if exceeded[0].startswith('records.') else 'result-payload-capacity-exhausted')
    vectors.append(dict(name=name, usage=usage, delta=delta, expected=dict(fits=not exceeded, exceeded=exceeded, reason=reason)))

for resource in ('records', 'payloads'):
    for scope, cap in (('assignment', 67108864), ('run', 134217728), ('workspace', 268435456)):
        for offset in (-1, 0, 1):
            usage, delta = zero(), zero()
            usage[resource][scope] = str(cap - 1)
            delta[resource][scope] = str(1 + offset)
            emit(f'{resource}.{scope}: cap{offset:+d}', usage, delta, [f'{resource}.{scope}'] if offset == 1 else [])

# Hierarchically consistent shared-scope witnesses, with exact one-byte deltas.
for resource in ('records', 'payloads'):
    for scope, values in (('run', (1, 134217728, 134217728)), ('workspace', (1, 2, 268435456))):
        usage, delta = zero(), zero()
        usage[resource] = dict(zip(('assignment', 'run', 'workspace'), map(str, values)))
        delta[resource] = dict.fromkeys(('assignment', 'run', 'workspace'), '1')
        emit(f'{resource}: peer filled {scope}', usage, delta, [f'{resource}.{scope}'])

usage, delta = zero(), zero()
for r in usage:
    usage[r] = dict(assignment='67108864', run='134217728', workspace='268435456')
emit('zero delta at all six ceilings (arithmetic only)', usage, delta, [])
usage, delta = zero(), zero()
usage['records']['assignment'] = '67108864'
usage['payloads']['assignment'] = '67108864'
delta['records']['assignment'] = delta['payloads']['assignment'] = '1'
emit('both independent resources fail: record reason wins', usage, delta, ['records.assignment', 'payloads.assignment'])
for r in ('records', 'payloads'):
    usage, delta = zero(), zero()
    usage[r]['assignment'] = '67108864'
    delta[r]['assignment'] = '1'
    emit(f'no borrowing from empty other pool: {r}', usage, delta, [f'{r}.assignment'])
print(json.dumps(dict(evidenceKind='isolated-arithmetic-not-source-valid-publication', vectors=vectors), indent=2))
