#!/usr/bin/env python3
"""Export original food facts through our vanilla-API oracle in a fresh instance."""
import argparse
import os
from pathlib import Path
import tempfile
import uuid
import neoforge_oracle as oracle


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--jar', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    if len(os.sched_getaffinity(0)) > 6 or os.getpriority(os.PRIO_PROCESS, 0) < 10:
        parser.error('run with taskset -c 0-5 nice -n 10')
    out = args.out.resolve()
    if not out.is_relative_to(oracle.ROOT / '.local'):
        parser.error('prepared facts must stay under repository .local/')
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='farmer-food-oracle-') as scratch:
        for key in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            path = Path(scratch) / key
            path.mkdir()
            os.environ[key] = str(path)
        os.environ.update(HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                          JAVA_TOOL_OPTIONS='-XX:ActiveProcessorCount=6')
        home = oracle.install()
        exporter = oracle.build(home)
        home, run = oracle.instance('farmer-food-' + uuid.uuid4().hex, [args.jar], [exporter])
        out.unlink(missing_ok=True)
        seconds, log = oracle.start(home, run, {'horizon.oracle.out': out,
                                               'horizon.oracle.namespaces': 'farmersdelight'}, timeout=90)
        if not out.exists():
            raise RuntimeError(f'oracle failed: {run}\n{log[-2000:]}')
        print(f'Exported food facts in {seconds:.2f}s; raw output {out}')


if __name__ == '__main__':
    main()
