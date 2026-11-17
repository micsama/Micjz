-- v7 → v8：表结构不变，分类重整为「一级 + 带 emoji 的二级」，模型也可以只选一级。
-- 按 key 对应：改 key 的保留原 id；去掉的分类把账目和识别留存挪到新位置后删除。
-- 网页上自己加的分类不动，排在这些分类后面。

UPDATE categories SET key = 'drink' WHERE key = 'coffee';
UPDATE categories SET key = 'trip' WHERE key = 'rail';
UPDATE categories SET key = 'vehicle' WHERE key = 'fuel';
UPDATE categories SET key = 'membership' WHERE key = 'subscription';

CREATE TEMP TABLE merged (old TEXT PRIMARY KEY, new TEXT NOT NULL) STRICT;
INSERT INTO merged VALUES
  ('dining', 'meal'), ('parking', 'vehicle'), ('property', 'rent'),
  ('treat', 'social'), ('shopping', 'daily');

SELECT '挪动账目 ' || m.old || ' → ' || m.new || '：' || count(e.id) || ' 条'
FROM merged m JOIN categories c ON c.key = m.old
LEFT JOIN entries e ON e.category_id = c.id
GROUP BY m.old;

UPDATE entries SET category_id = (
  SELECT n.id FROM categories o JOIN merged m ON m.old = o.key JOIN categories n ON n.key = m.new
  WHERE o.id = entries.category_id)
WHERE category_id IN (SELECT c.id FROM categories c JOIN merged m ON m.old = c.key);

UPDATE recognitions SET origin = json_set(origin, '$.category_id', (
  SELECT n.id FROM categories o JOIN merged m ON m.old = o.key JOIN categories n ON n.key = m.new
  WHERE o.id = json_extract(recognitions.origin, '$.category_id')))
WHERE json_extract(origin, '$.category_id') IN (SELECT c.id FROM categories c JOIN merged m ON m.old = c.key);

UPDATE categories SET position = position + 100;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🍚 饮食', 'food', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'food'), '🍱 三餐', 'meal', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'food'), '🍓 零食水果', 'snack', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'food'), '🧋 饮料咖啡', 'drink', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🚇 交通', 'transport', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'transport'), '🚌 公交地铁', 'transit', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'transport'), '🚕 打车', 'taxi', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'transport'), '🚄 火车飞机', 'trip', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'transport'), '🛵 汽车电驴', 'vehicle', 4)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🏠 住房', 'housing', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'housing'), '🔑 房租物业', 'rent', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'housing'), '💡 水电燃气', 'utilities', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'housing'), '📶 话费网费', 'telecom', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🧻 日用', 'daily', 4)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'daily'), '🧴 日用消耗', 'consumable', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'daily'), '💄 个护美妆', 'care', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'daily'), '🛏️ 家居家纺', 'home', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '👕 服饰', 'clothing', 5)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'clothing'), '👔 衣服', 'clothes', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'clothing'), '👟 鞋包', 'shoes_bags', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'clothing'), '🧢 配饰', 'adornment', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '💻 数码', 'digital', 6)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'digital'), '📱 设备', 'device', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'digital'), '🎧 配件外设', 'peripheral', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🎮 娱乐', 'fun', 7)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'fun'), '🕹️ 游戏', 'game', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'fun'), '🎬 影音演出', 'show', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'fun'), '🧳 旅行', 'travel', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '💳 会员', 'membership', 8)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'membership'), '🤖 AI 工具', 'ai', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'membership'), '📺 影音会员', 'media', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'membership'), '☁️ 云和软件', 'software', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🎁 人情', 'social', 9)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'social'), '🧧 红包', 'redpacket', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'social'), '🎀 礼物', 'gift', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🏥 医疗', 'medical', 10)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'medical'), '🩺 看病', 'clinic', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', (SELECT id FROM categories WHERE key = 'medical'), '💊 药品', 'medicine', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '✨ 其他', 'other', 11)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '💰 工资', 'salary', 1)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '🏆 奖金', 'bonus', 2)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '🧧 红包', 'redpacket_in', 3)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '↩️ 退款', 'refund', 4)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '📈 理财', 'investment', 5)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '✨ 其他', 'other_in', 6)
  ON CONFLICT(key) DO UPDATE SET parent_id = excluded.parent_id, name = excluded.name, position = excluded.position;

DELETE FROM categories WHERE key IN (SELECT old FROM merged);
DROP TABLE merged;
