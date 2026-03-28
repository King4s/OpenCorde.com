path = '/home/mb/opencorde/client/src/routes/servers/[serverId]/+layout.svelte'
with open(path) as f:
    c = f.read()
old = chr(9)*5 + 'goto();'
new = chr(9)*5 + "goto('/servers/' + sid + '/channels/' + next.id);"
c = c.replace(old, new)
with open(path, 'w') as f:
    f.write(c)
found = "goto('/servers/'" in c
print('OK' if found else 'FAIL')
