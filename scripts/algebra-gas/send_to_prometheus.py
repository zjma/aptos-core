#!/usr/bin/env python3

import argparse
from glob import glob
from load_bench_ns import main as load_bench_ns
import logging
import subprocess
import time
from pathlib import Path

def invoke(cmd, **kwargs):
    logging.getLogger(f'INVOKE').info(cmd)
    proc = subprocess.run(cmd, shell=True)
    logging.getLogger(f'INVOKE').info(f'exit_code={proc.returncode}')
    if not kwargs.get('allow_non_zero', False):
        assert proc.returncode == 0
    return proc.returncode

CRITERION_ROOT = Path('target/criterion')

def get_operation_name_by_bench_path(bench_path):
    rel_path = Path(bench_path).relative_to(CRITERION_ROOT)
    result = str(rel_path).replace('/', '.')
    return result

def load_and_send(bench_path):
    print(f'bench_path={bench_path}')
    cur_time = load_bench_ns(bench_path)
    if cur_time == None: return
    operation_name = get_operation_name_by_bench_path(bench_path)
    invoke(f'echo "ns_executing {cur_time}" | curl --data-binary @- http://ec2-35-91-8-165.us-west-2.compute.amazonaws.com:9091/metrics/job/some_job/operation/{operation_name}/machine_type/gcp.n2-standard-16')

def main(bench_patterns):
    for bench_pattern in bench_patterns:
        for bench_path in glob(bench_pattern):
            load_and_send(bench_path)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('bench_patterns', nargs='+')
    args = parser.parse_args()
    main(args.bench_patterns)

