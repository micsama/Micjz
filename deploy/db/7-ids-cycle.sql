-- v6 → v7：识别留存的草稿只认 id；记账周期允许「上月 N 号起」（负数）。
-- 必须在给分类补 emoji 之前跑：名字按转换当时的目录换成 id，对不上的分类、账户为 null，标签去掉。

UPDATE recognitions SET origin = json_remove(
  json_set(origin,
    '$.category_id', (
      SELECT coalesce(s.id, c.id) FROM categories c
      LEFT JOIN categories s
        ON s.parent_id = c.id AND s.name = json_extract(origin, '$.subcategory')
      WHERE c.kind = json_extract(origin, '$.kind') AND c.parent_id IS NULL
        AND c.name = json_extract(origin, '$.category')
        AND (json_extract(origin, '$.subcategory') IS NULL OR s.id IS NOT NULL)),
    '$.account_id', (SELECT id FROM accounts WHERE name = json_extract(origin, '$.account')),
    '$.tag_ids', json((
      SELECT json_group_array(t.id) FROM json_each(origin, '$.tags') j
      JOIN tags t ON t.name = j.value))),
  '$.category', '$.subcategory', '$.account', '$.tags');

SELECT '识别留存对不上的分类 ' || count(*) FILTER (WHERE json_extract(origin, '$.category_id') IS NULL)
    || ' 条，账户 ' || count(*) FILTER (WHERE json_extract(origin, '$.account_id') IS NULL) || ' 条'
FROM recognitions;

CREATE TABLE settings_new (
  id           INTEGER PRIMARY KEY CHECK (id = 1),
  cycle_day    INTEGER NOT NULL DEFAULT 1
               CHECK (cycle_day BETWEEN -28 AND 28 AND cycle_day NOT IN (-1, 0)),
  budget_cents INTEGER CHECK (budget_cents > 0)
) STRICT;
INSERT INTO settings_new SELECT id, cycle_day, budget_cents FROM settings;
DROP TABLE settings;
ALTER TABLE settings_new RENAME TO settings;
