输入音频 
  ↓
自动适配采样率和声道(设备实际支持的)
  ↓
16位PCM采集
  ↓
重采样处理(如果需要)
  → 采样率: 16000Hz
  → 声道数: 1
  → 位深度: 16位PCM
  ↓
Opus编码
  ↓
输出Opus数据


TODO：需要检查 packages/core/src/adapters/node/node-audio-recorder.js

