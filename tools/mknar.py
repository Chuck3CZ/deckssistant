import os, sys, struct

def s(b):
    if isinstance(b, str):
        b = b.encode()
    return struct.pack('<Q', len(b)) + b + b'\0' * ((8 - len(b) % 8) % 8)

def dump(p, rel=''):
    out = s('(')
    if os.path.islink(p):
        raise SystemExit('symlink not expected')
    if os.path.isdir(p):
        out += s('type') + s('directory')
        for n in sorted(os.listdir(p)):
            out += s('entry') + s('(') + s('name') + s(n) + s('node') + dump(os.path.join(p, n), rel + '/' + n) + s(')')
    else:
        out += s('type') + s('regular')
        # Only the launcher under bin/ may be executable; the device rejects any other executable file.
        if '/bin/' in rel:
            out += s('executable') + s('')
        out += s('contents') + s(open(p, 'rb').read())
    return out + s(')')

root, dest = sys.argv[1], sys.argv[2]
open(dest, 'wb').write(s('nix-archive-1') + dump(root))
