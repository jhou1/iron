# ── 通用 ──
app-name = IronCLI
app-about = 记录你的训练

# ── 热力图 ──
heatmap-less = 少
heatmap-more = 多
heatmap-mon = 一
heatmap-tue = 二
heatmap-wed = 三
heatmap-thu = 四
heatmap-fri = 五
heatmap-sat = 六
heatmap-sun = 日
heatmap-jan = 1月
heatmap-feb = 2月
heatmap-mar = 3月
heatmap-apr = 4月
heatmap-may = 5月
heatmap-jun = 6月
heatmap-jul = 7月
heatmap-aug = 8月
heatmap-sep = 9月
heatmap-oct = 10月
heatmap-nov = 11月
heatmap-dec = 12月

# ── 仪表盘 ──
dashboard-title = iron
dashboard-logo-text = 训练日志
dashboard-last-14-days = 最近 14 天
dashboard-last-7-days = 最近 7 天
dashboard-goals = 目标
dashboard-quotes = 语录
dashboard-no-quotes = 还没有语录 - 按 Q 添加
dashboard-no-entries = 最近 14 天没有记录
dashboard-sessions = { $count } 次训练
dashboard-total-volume = { $value } 公斤
dashboard-total-reps = { $value } 次
dashboard-total-distance = { $value } 公里
dashboard-total-duration = { $value } 分钟
dashboard-sets-metric = { $sets } 组, { $total } { $label }
dashboard-press-g = 按 [g] 添加目标
dashboard-press-a-goal = 按 [a] 添加目标
dashboard-date-prompt = 日期 (YYYY-MM-DD)：
dashboard-delete-confirm = 删除？(y/n)
dashboard-quotes-count = 语录 ({ $count })
dashboard-no-quotes-modal = 没有语录 - 按 [a] 添加
dashboard-hrv-label = HRV
dashboard-hrv-edit-hint = [v] 编辑
dashboard-hrv-record-hint = [v] 记录
dashboard-hrv-input-hint = (0-100, Enter 保存, Esc 取消)

# ── 目标 ──
goals-title = 目标

# ── 记录条目 ──
log-select-practice = 选择练习项目
log-press-filter = 按 / 过滤
log-weight-label = 重量（公斤）：
log-reps-label = 次数：
log-distance-label = 距离（公里）：
log-duration-label = 时长（分钟）：
log-note-label = 备注：
log-date-label = 日期：
log-date-confirm-hint = [Enter] 确认  [D] 编辑
log-date-change-hint = [D] 修改
log-date-edit-hint = （YYYY-MM-DD，按 Enter 确认）
log-set-line = 第 { $number } 组：{ $data }
log-sets-total = 组数：{ $sets }  合计：{ $total } { $label }
log-sets-total-reps = 组数：{ $sets }  合计：{ $total } { $label }  次数：{ $reps }
log-add-note-title = 记录 { $name } — 添加备注
log-sets-logged = 已记录 { $count } 组
log-total-value = 合计：{ $total } { $label }
log-note-optional = 备注（可选）
log-warmup-label = 热身
log-cooldown-label = 放松
log-warmup-cooldown-title = 记录 { $name } — 热身与放松

# ── 历史 ──
history-title = 历史
history-col-date = 日期
history-col-practice = 练习
history-col-volume = 量
history-no-entries = 暂无记录
history-entry = { $date }  { $name }  { $sets } 组  { $total } { $label }
history-set-weighted = #{ $number }  { $weight }公斤 x { $reps }
history-set-bodyweight = #{ $number }  { $reps } 次
history-set-distance = #{ $number }  { $distance } 公里
history-set-endurance = #{ $number }  { $duration } 分钟
history-note = 备注：{ $note }
history-warmup = 热身：{ $text }
history-cooldown = 放松：{ $text }
history-delete-confirm = 删除此记录？
history-summary =      总次数: { $reps } { $reps_label }  总容量: { $vol } { $vol_label }

# ── 趋势 ──
trends-title = 趋势 — 选择练习项目
trends-last-days = 最近 { $days } 天
trends-no-data = 此时段无数据。
trends-avg = 平均：{ $value }
trends-peak = 峰值：{ $value }
trends-trend = 趋势：{ $sign }{ $value }%

# ── 练习项目 ──
practices-title = 练习项目
practices-col-name = 名称
practices-col-type = 类型
practices-no-items = 还没有练习项目。按 'a' 添加。
practices-new-name = 新练习名称：
practices-select-type = 选择类型：
practices-rename = 重命名练习：
practices-delete-confirm = 删除 { $name }？
practices-delete-warning = 这将删除其所有记录。

# ── 练习类型标签 ──
practice-type-weighted = 重量x次数
practice-type-bodyweight = 次数
practice-type-distance = 距离
practice-type-endurance = 耐力

# ── 指标标签 ──
metric-kg-vol = 公斤量
metric-reps = 次
metric-km = 公里
metric-min = 分钟

# ── 组数据格式化 ──
set-weighted = { $weight } 公斤 x { $reps } 次
set-bodyweight = { $reps } 次
set-distance = { $distance } 公里
set-endurance = { $duration } 分钟

# ── 按键标签 ──
key-log = 记录
key-history = 历史
key-trends = 趋势
key-practices = 练习项目
key-goals = 目标
key-quotes = 语录
key-quit = 退出
key-navigate = 导航
key-filter = 过滤
key-select = 选择
key-back = 返回
key-add = 添加
key-edit = 编辑
key-delete = 删除
key-confirm = 确认
key-cancel = 取消
key-add-set = 添加组
key-save = 保存
key-date = 日期
key-del-last = 删除上组
key-add-goal = 添加目标
key-milestone = 里程碑
key-toggle = 切换
key-close = 关闭
key-window = 窗口
key-pick-practice = 选择练习
key-dashboard = 仪表盘
key-yes = 是
key-no = 否
key-hrv = HRV
key-next = 下一步
key-switch-field = 切换字段

# ── 命令行 ──
cli-export-about = 导出所有数据为 JSON
cli-import-about = 从 JSON 导入数据
cli-export-path-help = 输出文件路径（默认 ~/.ironcli/iron-export-YYYY-MM-DD.json）
cli-import-path-help = 输入文件路径
cli-export-complete = 导出完成。
cli-imported = 已导入 { $count } 条记录。
cli-exported-to = 已导出到 { $path }
