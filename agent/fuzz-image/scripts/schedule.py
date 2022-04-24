#!/usr/bin/env python
# Passed environment variables:
# - guid: Job Collection Guid
# - ID: id to prepend to fuzzer names
# - RAM
# - CPUS
# - TIMEOUT
# - FUZZ_DIR

from pathlib import Path
from zipfile import ZipFile
from configparser import ConfigParser, ExtendedInterpolation, BasicInterpolation
from subprocess import Popen, run, DEVNULL
from grpclib.server import Server, Stream

import os
import signal
import asyncio
import shutil
import logging
import sys

from grpc_handler import Processor

logging.getLogger().setLevel(logging.INFO)

class AFLInstance():
    count = 0
    def __init__(self, config, fuzz_dir, guid, id = "0", master = False):
        self.master = master
        self.env = dict(config["ENV"].items())
        self.fuzz_dir = fuzz_dir
        self.cmd = "afl-fuzz -i in -o out -Q {} {} -- {}/target".format(
            f"-M master_{id}" if self.master else f"-S slave_{id}",
            f"-T {id},guid:{guid}", # Tag injection vuln? lol
            fuzz_dir
        )

    async def start(self):
        self.popen = await asyncio.create_subprocess_shell(self.cmd, stdout = DEVNULL, stderr = DEVNULL, env = self.env, shell = True, cwd = self.fuzz_dir)
        AFLInstance.count += 1

    def kill(self):
        os.killpg(os.getpgid(self.popen.pid), signal.SIGTERM)

    async def wait(self):
        await self.popen.wait()

class Broker:
    def __init__(self):
        self.parse_env()
        Path(self.env["fuzz_dir"]).mkdir(parents = True, exist_ok = True)
        self.extract_files("/work/data/target.zip")
        self.extract_files("/work/data/corpus.zip")
        self.parse_config()
        self.rc = 0

    def parse_env(self):
        self.env = {
            "guid": os.environ.get("GUID"),
            "id": os.environ.get("ID"),
            "cpus": int(os.environ.get("CPUS")),
            "ram": os.environ.get("RAM"),
            "fuzz_dir": os.environ.get("FUZZ_DIR"),
        }

        if self.env["guid"] is None:
            logging.error("No GUID specified")
            exit(1)

        if self.env["id"] is None:
            logging.error("No ID specified")
            exit(1)

        if self.env["fuzz_dir"] is None:
            logging.error("No FUZZ_DIR specified")
            exit(1)

        if self.env["cpus"] is None:
            logging.error("No CPUS specified")
            exit(1)

    def extract_files(self, target):
        cp = run(args=["unzip", "-o", target, "-d", self.env["fuzz_dir"]], stdout = DEVNULL)
        if cp.returncode != 0:
            logging.error(f"Failed to unzip {target}")
            exit(1)

    def parse_config(self):
        self.config = ConfigParser(interpolation = ExtendedInterpolation())
        self.config.optionxform=str
        self.config.read(self.env["fuzz_dir"] + "/config.ini")

    async def schedule_fuzzers(self):
        self.instances = []

        x = AFLInstance(self.config, self.env["fuzz_dir"], self.env["guid"], self.env["id"], master = True)
        await x.start()
        self.instances.append(x)

        for x in range(1, self.env["cpus"]):
            instance = AFLInstance(self.config, self.env["fuzz_dir"], self.env["guid"], self.env["id"] + str(x))
            await instance.start()
            self.instances.append(instance)

    async def shutdown(self, signal):
        logging.info(f"Received signal {signal.name}")
        [x.kill() for x in self.instances]

    async def cancel_tasks(self):
        self.server.close()
        tasks = [t for t in asyncio.all_tasks() if t is not asyncio.current_task()]
        [task.cancel() for task in tasks]
        logging.info(f"Cancelling {len(tasks)} outstanding tasks")
        await asyncio.gather(*tasks, return_exceptions=True)
        await self.sync_corpus()

    async def wait_for_fuzzer_exit(self):
        awaitable = [asyncio.create_task(x.wait()) for x in self.instances]
        while True:
            logging.info("watching")
            done, _ = await asyncio.wait(awaitable, return_when=asyncio.FIRST_COMPLETED)

            for task in done:
                awaitable.remove(task)

            if len(awaitable) == 0:
                break

        for x in self.instances:
            if x.popen.returncode > 0:
                self.rc = x.popen.returncode
                break

    async def sync_corpus(self):
        out_dir = self.env["fuzz_dir"] + "/out/"
        src = out_dir + "master_{}".format(self.env["id"])
        dst = "/work/res/"
        await (await asyncio.create_subprocess_exec("rsync", "-rlpogtz", "--chown=1000:1000", "--exclude=README.txt", src, dst)).wait()
        await (await asyncio.create_subprocess_exec("rsync", "-rlpogtz", "--exclude=master_{}".format(self.env["id"]), dst, out_dir)).wait()
        logging.info(f"Sync done")

    async def watch_fuzzers(self):
        await self.wait_for_fuzzer_exit()
        await self.cancel_tasks()
        self.loop.stop()

    async def watch_corpus(self, time: int = 5*60):
        while True:
            await asyncio.sleep(time)
            await self.sync_corpus()

    async def handle_grpc(self, host: str = '0.0.0.0', port: int = 50051):
        await self.server.start(host, port)
        print(f'Serving on {host}:{port}')
        await self.server.wait_closed()

    def init_grpc(self):
        self.server = Server([Processor(self.env, self.config)])

    def init_signals(self):
        signals = (signal.SIGTERM, signal.SIGINT)
        for s in signals:
            self.loop.add_signal_handler(
                s, lambda s=s: asyncio.create_task(self.shutdown(s)))

    def run(self):
        self.loop = asyncio.get_event_loop()
        try:
            self.loop.run_until_complete(self.schedule_fuzzers())
            self.init_signals()
            self.init_grpc()
            self.loop.create_task(self.watch_fuzzers(), name="watch_fuzzers")
            self.loop.create_task(self.watch_corpus(), name="watch_corpus")
            self.loop.create_task(self.handle_grpc(), name="handle_grpc")
            self.loop.run_forever()
        finally:
            self.loop.close()
            logging.info("Successful shutdown")
        exit(self.rc)

def main():
    logging.info("started")
    broker = Broker()
    broker.run()

if __name__ == "__main__":
    main()
