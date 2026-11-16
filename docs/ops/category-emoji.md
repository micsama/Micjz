# 老库分类补 emoji（一次性数据修正）

## 现象

线上网页的分类图标全是汉字（「餐」「交」），没有 emoji。

## 原因

不是代码问题，是数据：

- 3d1e663 起分类的 emoji 是名称的一部分（`🍜 餐饮`），网页取名称开头的 emoji 当图标，没有就退回第一个字。
- `src/seed.sql` 的默认分类带 emoji，但只在建新库时执行。线上库建得早，分类名还是 `餐饮` 这类不带 emoji 的。
- 那次提交约定「已有库用 CLI 改名」，线上库一直没改。

## 修法

下面的 SQL 由 `src/seed.sql` 生成：按 `key` 找分类，只有名字仍是「默认名去掉 emoji」时才补上，自己改过名的分类不动。不改表结构，不影响账目。

已在本地演练：用 v5 表结构加 seed 建库、去掉所有 emoji、再把一个分类改成自定义名，执行后其余分类都补上，自定义名保留。

服务器上执行（先备份）：

```sh
cp data/micjz.db data/micjz.db.before-emoji
sqlite3 data/micjz.db < category-emoji.sql
micjz category list    # 核对；没补上的是改过名的，用 micjz category rename 自己补
```

不用重启服务，刷新网页即可。

```sql
-- 老库的分类名补上 emoji：按 key 找，只改名字还是「去掉 emoji 后的默认名」的，自己改过的不动。
BEGIN;
UPDATE categories SET name = '🍜 餐饮' WHERE key = 'food' AND name = '餐饮';
UPDATE categories SET name = '🍱 三餐' WHERE key = 'meal' AND name = '三餐';
UPDATE categories SET name = '☕️ 咖啡饮品' WHERE key = 'coffee' AND name = '咖啡饮品';
UPDATE categories SET name = '🍓 零食水果' WHERE key = 'snack' AND name = '零食水果';
UPDATE categories SET name = '🍲 聚餐' WHERE key = 'dining' AND name = '聚餐';
UPDATE categories SET name = '🚇 交通' WHERE key = 'transport' AND name = '交通';
UPDATE categories SET name = '🚌 公交地铁' WHERE key = 'transit' AND name = '公交地铁';
UPDATE categories SET name = '🚕 打车' WHERE key = 'taxi' AND name = '打车';
UPDATE categories SET name = '⛽️ 加油充电' WHERE key = 'fuel' AND name = '加油充电';
UPDATE categories SET name = '🅿️ 停车' WHERE key = 'parking' AND name = '停车';
UPDATE categories SET name = '✈️ 火车飞机' WHERE key = 'rail' AND name = '火车飞机';
UPDATE categories SET name = '🛍️ 购物' WHERE key = 'shopping' AND name = '购物';
UPDATE categories SET name = '🧻 日用' WHERE key = 'daily' AND name = '日用';
UPDATE categories SET name = '👕 服饰' WHERE key = 'clothing' AND name = '服饰';
UPDATE categories SET name = '🎧 数码' WHERE key = 'digital' AND name = '数码';
UPDATE categories SET name = '🛋️ 家居' WHERE key = 'home' AND name = '家居';
UPDATE categories SET name = '🏠 居住' WHERE key = 'housing' AND name = '居住';
UPDATE categories SET name = '🔑 房租' WHERE key = 'rent' AND name = '房租';
UPDATE categories SET name = '💡 水电燃气' WHERE key = 'utilities' AND name = '水电燃气';
UPDATE categories SET name = '📶 话费网费' WHERE key = 'telecom' AND name = '话费网费';
UPDATE categories SET name = '🏢 物业' WHERE key = 'property' AND name = '物业';
UPDATE categories SET name = '🎮 娱乐' WHERE key = 'fun' AND name = '娱乐';
UPDATE categories SET name = '📺 会员订阅' WHERE key = 'subscription' AND name = '会员订阅';
UPDATE categories SET name = '🕹️ 游戏' WHERE key = 'game' AND name = '游戏';
UPDATE categories SET name = '🎬 电影演出' WHERE key = 'show' AND name = '电影演出';
UPDATE categories SET name = '🧳 旅行' WHERE key = 'travel' AND name = '旅行';
UPDATE categories SET name = '🏥 医疗' WHERE key = 'medical' AND name = '医疗';
UPDATE categories SET name = '💊 药品' WHERE key = 'medicine' AND name = '药品';
UPDATE categories SET name = '🩺 门诊' WHERE key = 'clinic' AND name = '门诊';
UPDATE categories SET name = '🎁 人情' WHERE key = 'social' AND name = '人情';
UPDATE categories SET name = '🧧 红包' WHERE key = 'redpacket' AND name = '红包';
UPDATE categories SET name = '🎀 礼物' WHERE key = 'gift' AND name = '礼物';
UPDATE categories SET name = '🍻 请客' WHERE key = 'treat' AND name = '请客';
UPDATE categories SET name = '✨ 其他' WHERE key = 'other' AND name = '其他';
UPDATE categories SET name = '💰 工资' WHERE key = 'salary' AND name = '工资';
UPDATE categories SET name = '🏆 奖金' WHERE key = 'bonus' AND name = '奖金';
UPDATE categories SET name = '🧧 红包' WHERE key = 'redpacket_in' AND name = '红包';
UPDATE categories SET name = '↩️ 退款' WHERE key = 'refund' AND name = '退款';
UPDATE categories SET name = '📈 理财' WHERE key = 'investment' AND name = '理财';
UPDATE categories SET name = '✨ 其他' WHERE key = 'other_in' AND name = '其他';
COMMIT;
```
