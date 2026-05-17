# Export/Import Spec

CLI commands for backing up and restoring all application data as JSON.

## Commands

```
iron export [path]    # Export to JSON file
iron import <path>    # Import from JSON file
```

## Export

### Default path
`~/.iron/iron-export-YYYY-MM-DD.json`

### JSON Schema (version 3)

```json
{
  "version": 3,
  "exported_at": "ISO 8601 datetime",
  "practices": [
    {
      "id": 1,
      "name": "Deadlift",
      "type": "weighted",
      "created_at": "YYYY-MM-DD",
      "active": true
    }
  ],
  "logs": [
    {
      "id": 1,
      "practice": "Deadlift",
      "logged_at": "YYYY-MM-DD HH:MM:SS.fff",
      "note": "optional",
      "warm_up": "optional",
      "cool_down": "optional",
      "sets": [
        { "set_number": 1, "weight": 60.0, "reps": 5 }
      ]
    }
  ],
  "goals": [
    {
      "title": "Goal title",
      "position": 0,
      "completed": false,
      "completed_at": null,
      "milestones": [
        {
          "title": "Milestone",
          "completed": true,
          "position": 0,
          "completed_at": "datetime"
        }
      ]
    }
  ],
  "quotes": [
    { "text": "Quote text", "position": 0 }
  ],
  "daily_metrics": [
    { "date": "YYYY-MM-DD", "hrv": 42 }
  ]
}
```

### Data Sources

- `practices`: all practices (including inactive)
- `logs`: all logs across all time, all practices
- `goals`: all goals with milestones
- `quotes`: all quotes
- `daily_metrics`: all daily metrics records

## Import

### Behavior

| Entity | Strategy |
|--------|----------|
| Practices | Create if missing (match by name). Infer type from set data if needed. |
| Logs | Skip duplicates (matched by practice name + logged_at). Import new logs with sets. |
| Goals | **Replace all** — delete existing goals, recreate from import data |
| Quotes | **Replace all** — delete existing quotes, recreate from import data |
| Daily metrics | **Upsert** by date — insert new, update existing |

### Practice Type Inference

When importing a log for a practice not in the database, the type is inferred from set fields:
- weight + reps present -> weighted
- reps only -> bodyweight
- distance present -> distance
- duration present -> endurance

### Deduplication

A log is considered a duplicate if `log_exists(practice_name, logged_at)` returns true. Duplicate logs are silently skipped.

### Error Handling

- Invalid JSON: error message, abort
- Missing required fields: error message, abort
- Partial import on practice creation failure: logged but continues
