-- v3 → v4：加单行设置表（记账周期、每期预算）。
-- 防重：表已存在、默认行已有时什么都不做，重复执行结果相同。
CREATE TABLE IF NOT EXISTS settings (
  id           INTEGER PRIMARY KEY CHECK (id = 1),
  cycle_day    INTEGER NOT NULL DEFAULT 1 CHECK (cycle_day BETWEEN 1 AND 28),
  budget_cents INTEGER CHECK (budget_cents > 0)
);
INSERT OR IGNORE INTO settings (id) VALUES (1);
