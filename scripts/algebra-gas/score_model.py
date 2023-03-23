#!/usr/bin/env python3

import argparse
import json
import load_bench_datapoints
import load_bench_ns
from math import ceil, floor, log2
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
from pprint import pprint

class ArkMsmModel:
    def __init__(self, scalar_field_bitlen, addition_cost, doubling_cost):
        self.scalar_field_bitlen = scalar_field_bitlen
        self.addition_cost = addition_cost
        self.doubling_cost = doubling_cost
    def predict(self, x):
        window_size = 3 if x < 32 else (ceil(log2(x)) * 69 // 100 + 2)
        num_windows = ceil(self.scalar_field_bitlen / window_size)
        num_buckets = 1 << window_size
        cost = self.addition_cost * (x+num_buckets+1) * num_windows + self.doubling_cost * (window_size*num_windows)
        return cost
class NonLinearMsmModel:
    def __init__(self):
        pass
    def predict(self, x):
        gas = 9420000 * x//floor(log2(x+1)) + 6000 * x
        ns = gas / 205.41
        return ns
class TwoPhasedMsmModel:
    def __init__(self):
        pass
    def predict(self, x):
        ns = (11108.570175438595*x+40427.833333333125) if x<190 else (6703.821179361177*x+1244411.1577395604)
        return ns
class Sha256Model:
    def __init__(self):
        pass
    def predict(self, x):
        gas = 1000 * x + 60000
        ns = gas / 205.41
        return ns
class LinearModel:
    def __init__(self, k, b):
        self.k = k
        self.b = b
    def predict(self, x):
        return self.k*x+self.b

def load_model(model_path):
    if model_path == 'builtin_ark_bls12_381_g1_affine_msm':
        return ArkMsmModel(255, load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_add'), load_bench_ns.main('target/criterion/ark_bls12_381/g1_proj_double'))
    if model_path == 'builtin_ark_bls12_381_g2_affine_msm':
        return ArkMsmModel(255, load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_add'), load_bench_ns.main('target/criterion/ark_bls12_381/g2_proj_double'))
    if model_path == 'builtin_non_linear_msm':
        return NonLinearMsmModel()
    if model_path == 'builtin_sha256':
        return Sha256Model()
    if model_path == '2_phased_msm':
        return TwoPhasedMsmModel()
    obj = json.loads(Path(model_path).read_text())
    return LinearModel(obj['k'], obj['b'])

class PointStat:
    def __init__(self, x, y, y_hat):
        self.x = x
        self.y = y
        self.y_hat = y_hat
        self.est_rate = y_hat / y
    def __repr__(self):
        return f'x={self.x}, y={self.y}, y_hat={self.y_hat}, est_rate={self.est_rate}'

def main(dataset_path, model_path):
    datapoints = json.loads(Path(dataset_path).read_text())
    x_values, y_values = zip(*datapoints)
    model = load_model(model_path)
    y_hat_values = [model.predict(x) for x in x_values]
    X = np.array(x_values)
    Y = np.array(y_values)
    Y_hat = np.array(y_hat_values)
    n = len(X)
    stats = [PointStat(X[i], Y[i], Y_hat[i]) for i in range(n)]
    stats.sort(key=lambda st:st.est_rate)
    return X, Y, Y_hat, stats

    if plot:
        plt.plot(X, Y, 'o', label='ns sampled', markersize=2)
        plt.plot(X, model(X), 'r', label='ns predicted')
        plt.legend()
        plt.show(block=True)

if __name__=='__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--dataset_path', required=True)
    parser.add_argument('--model_path', required=True)
    parser.add_argument('--plot', action='store_true')
    args = parser.parse_args()
    X, Y, Y_hat, stats_sorted_by_est_rate = main(args.dataset_path, args.model_path)
    pprint(stats_sorted_by_est_rate)
    if args.plot:
        plt.plot(X, Y, 'o', label='ns sampled', markersize=2)
        plt.plot(X, Y_hat, 'r', label='ns predicted')
        plt.legend()
        plt.show(block=True)
