"""Scratch-only real-game driver for port conservation fixtures (dev tool)."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


class Fixture:
    def __init__(self, args, mode, mods):
        self.args, self.mode, self.mods = args, mode, mods
        self.temp = tempfile.TemporaryDirectory(prefix='horizon-port-fixture-')
        self.root = Path(self.temp.name)
        self.env = dict(os.environ, HORIZON_LAUNCHER_NO_SELF_UPDATE='1', HORIZON_NO_AUDIO='1',
                        HORIZON_TELEMETRY_URL='off', RAYON_NUM_THREADS='6')
        for name in ['HORIZON_HOME', 'HORIZON_LAUNCHER_HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME']:
            self.env[name] = tempfile.mkdtemp(prefix=name.lower(), dir=self.root)
        self.socket = self.root / 'ctl.sock'
        self.process = None
        self.logs = []

    def __enter__(self):
        return self

    def __exit__(self, *_):
        try:
            if self.process is not None:
                try:
                    self.stop()
                except (AssertionError, subprocess.TimeoutExpired):
                    self.process.kill()
                    self.process.wait()
        finally:
            self.args.out.parent.mkdir(parents=True, exist_ok=True)
            for index, log in enumerate(self.logs):
                target = self.args.out.parent / f'{self.args.out.stem}-{self.mode}-{index}.log'
                target.write_text(log.read_text(errors='replace'))
            self.temp.cleanup()

    def call(self, *words):
        print(self.mode, *words, flush=True)
        result = subprocess.run([str(self.args.bin_dir / 'horizon-ctl'), '--socket', str(self.socket),
                                 *map(str, words)], cwd=self.root, env=self.env,
                                capture_output=True, text=True, timeout=180)
        lines = result.stdout.strip().splitlines()
        reply = json.loads(lines[-1]) if lines else {'error': result.stderr.strip()}
        if result.returncode or 'error' in reply:
            raise AssertionError((words, reply))
        return reply

    def command(self, text):
        chat = self.call('command', text).get('chat', [])
        if any('<--[HERE]' in line for line in chat):
            raise AssertionError((text, chat))
        return next(iter(chat), '')

    def start(self):
        log = self.root / f'game-{len(self.logs)}.log'
        self.logs.append(log)
        argv = [str(self.args.bin_dir / 'horizon-client'), '--resources', str(self.args.resources),
                '--world', 'PortFixture', '--test-terrain', '--creative', '--render-distance', '2',
                '--offscreen', '--width', '320', '--height', '240', '--control-socket', str(self.socket)]
        for mod in self.mods:
            argv += ['--dev-mod', str(mod)]
        if self.mode == 'in-frame':
            argv.append('--in-frame')
        with log.open('w') as stream:
            self.process = subprocess.Popen(argv, cwd=self.root, env=self.env, stdout=stream, stderr=stream)
        deadline = time.monotonic() + 180
        while not self.socket.exists():
            if self.process.poll() is not None or time.monotonic() > deadline:
                raise AssertionError(log.read_text(errors='replace')[-6000:])
            time.sleep(0.2)
        self.call('wait-ready', '180')
        self.command('tick freeze')

    def stop(self):
        self.call('quit')
        self.process.wait(timeout=45)
        assert self.process.returncode == 0
        self.process = None
        self.socket.unlink(missing_ok=True)

    def step(self, ticks):
        self.command(f'tick step {ticks}')
        time.sleep(ticks / 20 + 1)

    def slots(self, pos):
        return self.call('container', *pos)['slots']

    def total(self, positions, item):
        return sum(stack['count'] for pos in positions for stack in self.slots(pos)
                   if stack and stack['item'] == item)

    def replace(self, pos, slot, item):
        return self.command(f'item replace block {" ".join(map(str, pos))} container.{slot} with {item}')

    def components(self, pos, slot=0):
        text = self.command(f'data get block {" ".join(map(str, pos))} Items[{slot}].components')
        assert '{' in text, text
        return text[text.index('{'):]
