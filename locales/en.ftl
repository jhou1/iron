# ── Common ──
app-name = iron
app-about = Track your training

# ── Heatmap ──
heatmap-less = Less
heatmap-more = More
heatmap-mon = Mo
heatmap-tue = Tu
heatmap-wed = We
heatmap-thu = Th
heatmap-fri = Fr
heatmap-sat = Sa
heatmap-sun = Su
heatmap-jan = Jan
heatmap-feb = Feb
heatmap-mar = Mar
heatmap-apr = Apr
heatmap-may = May
heatmap-jun = Jun
heatmap-jul = Jul
heatmap-aug = Aug
heatmap-sep = Sep
heatmap-oct = Oct
heatmap-nov = Nov
heatmap-dec = Dec

# ── Dashboard ──
dashboard-title = iron
dashboard-logo-text = Iron
dashboard-last-14-days = Last 14 Days
dashboard-last-7-days = Last 7 Days
dashboard-goals = Goals
dashboard-quotes = Quotes
dashboard-no-quotes = No quotes yet — press Q to add one
dashboard-no-entries = No entries in the last 14 days
dashboard-sessions = { $count } sessions
dashboard-total-volume = { $value } kg
dashboard-total-reps = { $value } reps
dashboard-total-distance = { $value } km
dashboard-total-duration = { $value } min
dashboard-sets-metric = { $sets } sets, { $total } { $label }
dashboard-press-g = Press [g] to add goals
dashboard-press-a-goal = Press [a] to add a goal
dashboard-date-prompt = Date (YYYY-MM-DD):
dashboard-delete-confirm = Delete? (y/n)
dashboard-quotes-count = Quotes ({ $count })
dashboard-no-quotes-modal = No quotes — press [a] to add one
dashboard-hrv-label = HRV
dashboard-hrv-edit-hint = [v] edit
dashboard-hrv-record-hint = [v] record
dashboard-hrv-input-hint = (0-100, Enter to save, Esc to cancel)
dashboard-quit-confirm = Quit iron?

# ── Goals ──
goals-title = Goals

# ── Log Entry ──
log-select-practice = Select Practice
log-press-filter = Press / to filter
log-weight-label = Weight (kg):
log-reps-label = Reps:
log-distance-label = Distance (km):
log-duration-label = Duration (min):
log-note-label = Note:
log-date-label = Date:
log-date-confirm-hint = [Enter] confirm  [D] edit
log-date-change-hint = [D] to change
log-date-edit-hint = (YYYY-MM-DD, Enter to confirm)
log-set-line = Set { $number }: { $data }
log-sets-total = Sets: { $sets }  Total: { $total } { $label }
log-sets-total-reps = Sets: { $sets }  Total: { $total } { $label }  Reps: { $reps }
log-add-note-title = Log { $name } — Add Note
log-sets-logged = { $count } sets logged
log-total-value = Total: { $total } { $label }
log-note-optional = Note (optional)
log-warmup-label = Warm-up
log-cooldown-label = Cool-down
log-warmup-cooldown-title = Log { $name } — Warm-up & Cool-down

# ── History ──
history-title = History
history-col-date = Date
history-col-practice = Practice
history-col-volume = Volume
history-no-entries = No entries yet
history-entry = { $date }  { $name }  { $sets } sets  { $total } { $label }
history-set-weighted = #{ $number }  { $weight }kg x { $reps }
history-set-bodyweight = #{ $number }  { $reps } reps
history-set-distance = #{ $number }  { $distance } km
history-set-endurance = #{ $number }  { $duration } min
history-note = Note: { $note }
history-warmup = Warm-up: { $text }
history-cooldown = Cool-down: { $text }
history-delete-confirm = Delete this entry?
history-summary =      Total reps: { $reps } { $reps_label }  Volume: { $vol } { $vol_label }

# ── Trends ──
trends-title = Trends — Select Practice
trends-last-days = Last { $days } days
trends-no-data = No data for this period.
trends-avg = Avg: { $value }
trends-peak = Peak: { $value }
trends-trend = Trend: { $sign }{ $value }%

# ── Practices ──
practices-title = Practices
practices-col-name = Name
practices-col-type = Type
practices-no-items = No practices yet. Press 'a' to add one.
practices-new-name = New practice name:
practices-select-type = Select type:
practices-rename = Rename practice:
practices-delete-confirm = Delete { $name }?
practices-delete-warning = This removes all its logs.
practices-col-status = Status
practices-status-active = active
practices-status-inactive = inactive

# ── Practice type labels ──
practice-type-weighted = weightxreps
practice-type-bodyweight = reps
practice-type-distance = distance
practice-type-endurance = duration

# ── Metric labels ──
metric-kg-vol = kg vol
metric-reps = reps
metric-km = km
metric-min = min

# ── Set data formatting ──
set-weighted = { $weight } kg x { $reps } reps
set-bodyweight = { $reps } reps
set-distance = { $distance } km
set-endurance = { $duration } min

# ── Keyboard labels ──
key-log = Log
key-history = History
key-trends = Trends
key-practices = Practices
key-goals = Goals
key-quotes = Quotes
key-quit = Quit
key-navigate = Navigate
key-filter = Filter
key-select = Select
key-back = Back
key-add = Add
key-edit = Edit
key-delete = Delete
key-confirm = Confirm
key-cancel = Cancel
key-add-set = Add set
key-save = Save
key-date = Date
key-del-last = Del last
key-add-goal = Add goal
key-milestone = Milestone
key-toggle = Toggle
key-close = Close
key-window = Window
key-pick-practice = Pick practice
key-dashboard = Dashboard
key-yes = Yes
key-no = No
key-hrv = HRV
key-next = Next
key-switch-field = Switch field
key-undo = Undo
key-help = Help

# ── Status Messages ──
status-deleted-undo = Deleted. Press [u] to undo
status-restored = Restored
status-save-error = Error: { $msg }
status-delete-error = Delete failed: { $msg }

# ── Help ──
help-title = Keyboard Shortcuts

# ── Additional UI ──
quotes-delete-confirm = Delete this quote?
practices-delete-cascade-warning = This PERMANENTLY removes all logs. Consider [t] toggle inactive.
terminal-too-small = Please resize terminal (min { $w }×{ $h })

# ── CLI ──
cli-export-about = Export all data to JSON
cli-import-about = Import data from JSON
cli-export-path-help = Output file path (defaults to ~/.iron/iron-export-YYYY-MM-DD.json)
cli-import-path-help = Input file path
cli-export-complete = Export complete.
cli-imported = Imported { $count } logs.
cli-exported-to = Exported to { $path }
