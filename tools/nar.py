#!/usr/bin/env python3
"""Minimal Nix ARchive (NAR) reader/writer: `nar.py ls FILE`, `nar.py unpack FILE DIR`, `nar.py pack DIR FILE`."""
import os, stat, struct, sys


def w(out, b):
    if isinstance(b, str):
        b = b.encode()
    out.append(struct.pack("<Q", len(b)) + b + b"\0" * (-len(b) % 8))


def dump(out, path):
    w(out, "(")
    if os.path.islink(path):
        w(out, "type"); w(out, "symlink"); w(out, "target"); w(out, os.readlink(path))
    elif os.path.isdir(path):
        w(out, "type"); w(out, "directory")
        for name in sorted(os.listdir(path)):
            w(out, "entry"); w(out, "("); w(out, "name"); w(out, name); w(out, "node")
            dump(out, os.path.join(path, name))
            w(out, ")")
    else:
        w(out, "type"); w(out, "regular")
        if os.stat(path).st_mode & stat.S_IXUSR:
            w(out, "executable"); w(out, "")
        w(out, "contents"); w(out, open(path, "rb").read())
    w(out, ")")


def pack(src, dst):
    out = []
    w(out, "nix-archive-1")
    dump(out, src)
    open(dst, "wb").write(b"".join(out))


class Reader:
    def __init__(self, data):
        self.d, self.p = data, 0

    def s(self):
        n = struct.unpack_from("<Q", self.d, self.p)[0]
        v = self.d[self.p + 8:self.p + 8 + n]
        self.p += 8 + n + (-n % 8)
        return v

    def node(self, path, visit):
        assert self.s() == b"("
        assert self.s() == b"type"
        kind = self.s()
        if kind == b"directory":
            visit(path, "dir", None, False)
            while True:
                t = self.s()
                if t == b")":
                    return
                assert t == b"entry" and self.s() == b"(" and self.s() == b"name"
                name = self.s().decode()
                assert self.s() == b"node"
                self.node(path + "/" + name, visit)
                assert self.s() == b")"
        elif kind == b"symlink":
            assert self.s() == b"target"
            visit(path, "link", self.s(), False)
            assert self.s() == b")"
        else:
            t, ex = self.s(), False
            if t == b"executable":
                self.s(); ex = True; t = self.s()
            assert t == b"contents"
            visit(path, "file", self.s(), ex)
            assert self.s() == b")"


def main():
    cmd, a = sys.argv[1], sys.argv[2:]
    if cmd == "pack":
        pack(a[0], a[1]); return
    r = Reader(open(a[0], "rb").read())
    assert r.s() == b"nix-archive-1"
    def visit(path, kind, data, ex):
        if cmd == "ls":
            print(kind, "x" if ex else " ", len(data) if data is not None else "", path)
        elif cmd == "unpack":
            dest = os.path.join(a[1], path.lstrip("/"))
            if kind == "dir":
                os.makedirs(dest, exist_ok=True)
            elif kind == "link":
                os.symlink(data.decode(), dest)
            else:
                open(dest, "wb").write(data); os.chmod(dest, 0o755 if ex else 0o644)
    r.node("", visit)


main()
