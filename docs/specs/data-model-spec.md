# Data Model Spec

The core domain types used throughout the application. These are framework-agnostic and represent the business logic layer.

## Entities

### PracticeType (Enum)

Four types of training activity, each with a distinct metric:

| Type | Fields per Set | Metric | Unit Label |
|------|---------------|--------|------------|
| Weighted | weight (float), reps (int) | weight x reps | kg·vol |
| Bodyweight | reps (int) | reps | reps |
| Distance | distance (float) | distance | km |
| Endurance | duration (float) | duration | min |

Each type has two string representations:
- **UI label**: `weightxreps`, `reps`, `distance`, `duration`
- **Storage name**: `weighted`, `bodyweight`, `distance`, `endurance`

### Practice

A named training activity.

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK, auto-increment |
| name | string | unique, non-empty |
| practice_type | PracticeType | immutable after creation |
| created_at | datetime | auto-set on creation |
| active | boolean | default true, controls visibility |

Inactive practices are hidden from all screens except the practice management screen.

### Log

A single training session for one practice.

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK, auto-increment |
| practice_id | integer | FK -> Practice, cascade delete |
| logged_at | datetime | user-editable, default now |
| note | string? | optional free-text session note |
| warm_up | string? | optional warm-up description |
| cool_down | string? | optional cool-down description |

### Set

One effort within a log. Only the fields relevant to the practice type are populated.

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK, auto-increment |
| log_id | integer | FK -> Log, cascade delete |
| set_number | integer | 1-based ordering |
| weight | float? | Weighted only |
| reps | integer? | Weighted or Bodyweight |
| distance | float? | Distance only |
| duration | float? | Endurance only |

### SetData (Polymorphic Value)

In application code, a set's data is represented as a tagged union:

- `Weighted { weight: float, reps: int }` — metric = weight x reps
- `Bodyweight { reps: int }` — metric = reps
- `Distance { distance: float }` — metric = distance
- `Endurance { duration: float }` — metric = duration

### LogEntry (Composite View)

A denormalized view used for display. Joins Log + Practice + Sets.

| Field | Type |
|-------|------|
| log | Log |
| practice_name | string |
| practice_type | PracticeType |
| sets | list of Set |

Derived methods:
- `total_metric()` — sum of all sets' metric values
- `metric_label()` — human-readable unit from practice type

### Goal

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK |
| title | string | non-empty |
| completed | boolean | default false |
| position | integer | ordering (0-based, lower = higher) |
| created_at | datetime | auto-set |
| completed_at | datetime? | set when toggled complete |
| milestones | list of Milestone | lazy-loaded, one-to-many |

Progress = completed milestones / total milestones. If no milestones: 0% or 100% based on `completed`.

### Milestone

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK |
| goal_id | integer | FK -> Goal, cascade delete |
| title | string | non-empty |
| completed | boolean | default false |
| position | integer | ordering within goal |
| created_at | datetime | auto-set |
| completed_at | datetime? | set when toggled |

### Quote

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK |
| text | string | non-empty |
| position | integer | ordering |

Dashboard displays one random quote at a time.

### DailyMetrics

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK |
| date | string | YYYY-MM-DD, unique |
| hrv | integer? | 0-100 (Heart Rate Variability) |

Upsert by date — one record per day.

### Abbreviation

| Field | Type | Constraints |
|-------|------|------------|
| id | integer | PK |
| short | string | unique, case-insensitive |
| full_name | string | maps to a practice name |

Used by Quick Log to expand shorthand (e.g., "DL" -> "Deadlift").

## Relationships

```
Practice 1──* Log 1──* Set
Goal 1──* Milestone
```

Deletion cascades: Practice -> Logs -> Sets, Goal -> Milestones.

## Aggregate Queries

- `heatmap_counts(days)` — returns list of (date, session_count) for the last N days
- `aggregate_stats(days)` — returns total sessions, volume, reps, distance, duration over N days
- `list_logs_for_practice(practice_id, days)` — logs for one practice in a time window
- `log_exists(practice_name, logged_at)` — deduplication check for import
