"""Reads /proc: process trees, RSS/PSS, CPU ticks and context switches (summed over every thread)."""
import os

CLK_TCK = os.sysconf("SC_CLK_TCK")


def _read(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return None


def children_map():
    out = {}
    for name in os.listdir("/proc"):
        if not name.isdigit():
            continue
        stat = _read(f"/proc/{name}/stat")
        if not stat:
            continue
        # comm may contain spaces and parentheses: split after the last ')'.
        rest = stat[stat.rindex(")") + 2 :].split()
        out.setdefault(int(rest[1]), []).append(int(name))
    return out


def descendants(root):
    kids = children_map()
    found, todo = [], [root]
    while todo:
        pid = todo.pop()
        found.append(pid)
        todo.extend(kids.get(pid, []))
    return found


def comm(pid):
    c = _read(f"/proc/{pid}/comm")
    return c.strip() if c else ""


def pids_named(name):
    return [int(p) for p in os.listdir("/proc") if p.isdigit() and comm(int(p)) == name]


def rss_kb(pid):
    s = _read(f"/proc/{pid}/status") or ""
    for line in s.splitlines():
        if line.startswith("VmRSS:"):
            return int(line.split()[1])
    return 0


def pss_kb(pid):
    s = _read(f"/proc/{pid}/smaps_rollup") or ""
    for line in s.splitlines():
        if line.startswith("Pss:"):
            return int(line.split()[1])
    return 0


def memory(pids):
    return {"rss_kb": sum(rss_kb(p) for p in pids), "pss_kb": sum(pss_kb(p) for p in pids)}


def threads(pid):
    """{tid: (comm, ticks, switches)} for one process."""
    out = {}
    try:
        tids = os.listdir(f"/proc/{pid}/task")
    except OSError:
        return out
    for tid in tids:
        stat = _read(f"/proc/{pid}/task/{tid}/stat")
        status = _read(f"/proc/{pid}/task/{tid}/status")
        if not stat or not status:
            continue
        name = stat[stat.index("(") + 1 : stat.rindex(")")]
        rest = stat[stat.rindex(")") + 2 :].split()
        ticks = int(rest[11]) + int(rest[12])
        switches = 0
        for line in status.splitlines():
            if "ctxt_switches" in line:
                switches += int(line.split()[1])
        out[(pid, int(tid))] = (name, ticks, switches)
    return out


def snapshot(pids):
    snap = {}
    for pid in pids:
        snap.update(threads(pid))
    return snap


def delta(before, after, seconds):
    """CPU% (of one core) and context switches per second over threads that exist in both snapshots."""
    ticks = switches = 0
    by_name = {}
    for key, (name, t1, s1) in after.items():
        if key not in before:
            continue
        _, t0, s0 = before[key]
        ticks += t1 - t0
        switches += s1 - s0
        agg = by_name.setdefault(name, [0, 0])
        agg[0] += t1 - t0
        agg[1] += s1 - s0
    top = sorted(by_name.items(), key=lambda kv: -kv[1][1])[:6]
    return {
        "cpu_percent": round(ticks / CLK_TCK / seconds * 100, 3),
        "wakeups_per_s": round(switches / seconds, 2),
        "top_threads": [
            {"thread": n, "cpu_percent": round(v[0] / CLK_TCK / seconds * 100, 3), "switches_per_s": round(v[1] / seconds, 2)}
            for n, v in top
        ],
    }
