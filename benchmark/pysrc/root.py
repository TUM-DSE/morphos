from pathlib import Path
from os import path

TEST_ROOT = Path(__file__).parent.resolve().parent.resolve()
PROJECT_ROOT = TEST_ROOT.parent

QEMU_BUILD_DIR = "/scratch/okelmann/vmuxIO/qemu/bin" # TODO