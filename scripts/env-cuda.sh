export CUDA_PATH=$HOME/cuda-12.6
export PATH=$CUDA_PATH/bin:$HOME/local/usr/bin:$PATH
export CUDA_COMPUTE_CAP=89
export CPLUS_INCLUDE_PATH=$HOME/local/usr/include/c++/13:$HOME/local/usr/lib/gcc/x86_64-linux-gnu/13/include
export NVCC_PREPEND_FLAGS="-ccbin $HOME/local/usr/bin/g++-13"
export ZLUDA_HOME=$HOME/zluda/zluda
# 运行时：ZLUDA 驱动 shim + NVIDIA 链接库（curand）+ ROCm HIP 底层
export LD_LIBRARY_PATH=$HOME/zluda/zluda:$HOME/cuda-12.6/lib64:/opt/rocm/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
