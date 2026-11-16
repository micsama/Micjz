-- 新库的初始分类、账户、标签：建库时执行一次。
-- 之后以数据库为准，用 `micjz category|account|tag ...` 管理（见 readme.md）。
-- 每个分类都有 key（小写英文，全局唯一）；只有叶子的 key 会给模型。
-- position 是同级内的优先级，从 1 数。
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🍜 餐饮', 'food', 1);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🍱 三餐', 'meal', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🍜 餐饮';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '☕️ 咖啡饮品', 'coffee', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🍜 餐饮';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🍓 零食水果', 'snack', 3 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🍜 餐饮';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🍲 聚餐', 'dining', 4 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🍜 餐饮';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🚇 交通', 'transport', 2);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🚌 公交地铁', 'transit', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🚇 交通';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🚕 打车', 'taxi', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🚇 交通';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '⛽️ 加油充电', 'fuel', 3 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🚇 交通';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🅿️ 停车', 'parking', 4 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🚇 交通';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '✈️ 火车飞机', 'rail', 5 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🚇 交通';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🛍️ 购物', 'shopping', 3);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🧻 日用', 'daily', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🛍️ 购物';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '👕 服饰', 'clothing', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🛍️ 购物';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🎧 数码', 'digital', 3 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🛍️ 购物';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🛋️ 家居', 'home', 4 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🛍️ 购物';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🏠 居住', 'housing', 4);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🔑 房租', 'rent', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🏠 居住';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '💡 水电燃气', 'utilities', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🏠 居住';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '📶 话费网费', 'telecom', 3 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🏠 居住';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🏢 物业', 'property', 4 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🏠 居住';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🎮 娱乐', 'fun', 5);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '📺 会员订阅', 'subscription', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎮 娱乐';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🕹️ 游戏', 'game', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎮 娱乐';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🎬 电影演出', 'show', 3 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎮 娱乐';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🧳 旅行', 'travel', 4 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎮 娱乐';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🏥 医疗', 'medical', 6);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '💊 药品', 'medicine', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🏥 医疗';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🩺 门诊', 'clinic', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🏥 医疗';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '🎁 人情', 'social', 7);
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🧧 红包', 'redpacket', 1 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎁 人情';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🎀 礼物', 'gift', 2 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎁 人情';
INSERT INTO categories (kind, parent_id, name, key, position) SELECT 'expense', id, '🍻 请客', 'treat', 3 FROM categories WHERE kind = 'expense' AND parent_id IS NULL AND name = '🎁 人情';
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('expense', NULL, '✨ 其他', 'other', 8);
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '💰 工资', 'salary', 1);
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '🏆 奖金', 'bonus', 2);
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '🧧 红包', 'redpacket_in', 3);
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '↩️ 退款', 'refund', 4);
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '📈 理财', 'investment', 5);
INSERT INTO categories (kind, parent_id, name, key, position) VALUES ('income', NULL, '✨ 其他', 'other_in', 6);
INSERT INTO accounts (name, position) VALUES ('微信', 1);
INSERT INTO accounts (name, position) VALUES ('银行卡', 2);
INSERT INTO tags (name) VALUES ('恋爱');
