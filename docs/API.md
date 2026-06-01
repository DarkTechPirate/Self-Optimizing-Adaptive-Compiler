# Nyx API

Start server:
```bash
nyx serve --host 127.0.0.1 --port 8090
```

## Endpoints
- `GET /health`
- `POST /execute`
- `POST /analyze`
- `POST /optimize`
- `GET /metrics/recent?limit=35`
- `GET /learning/summary?limit=15`
- `GET /dashboard`

## Optimize request
```json
{
  "source": "def hot_path():\n    total = 0\n    for i in range(0, 50):\n        total = total + i\n    return total",
  "mode": "auto",
  "no_llm": true
}
```

## Optimize response highlights
- `input_format`: `nyx`, `python`, or `cpp`
- `source_normalized`: true when Python or C++ is normalized to Nyx syntax
- `selected_strategies`
- `strategy_scores`
- `reused_history`
- `speedup_ratio`
- `time_saved_us`
- `total_time_saved_us`
- `time_saved_today_us`
- `baseline_return_value`
- `optimized_return_value`
- `correctness_verified`

## Learning summary response highlights
- `learning_event_count`
- `strategies` (runs, average speedup, success rate)
- `total_time_saved_us`
- `time_saved_today_us`
