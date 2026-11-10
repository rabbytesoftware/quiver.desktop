import statistics


def summary(values):
    values = [v for v in values if v is not None]
    if not values:
        return None
    return {"median": round(statistics.median(values), 2), "min": round(min(values), 2), "max": round(max(values), 2), "n": len(values)}


def summarise_runs(runs):
    """Median/min/max for every numeric leaf that appears in all runs."""
    keys = {}
    for run in runs:
        for k, v in run.items():
            if isinstance(v, (int, float)):
                keys.setdefault(k, []).append(v)
    return {k: summary(v) for k, v in keys.items()}
