# 蓝图：截图记账后补采位置（表结构 v10）

状态：已实现（2026-11-23），待实机验收。表结构版本 9 → 10，城市 10 → 11。第一版只做快捷指令场景，网页手填不记位置。

## 目标

截图记账时顺手记下手机所在的位置，不加任何点击、菜单或提示（无感）。位置只是可选的补充：定位被拒、失败、超时，或者切到后台，都只会丢掉这一笔的位置，**不影响入账**。先把数据攒起来，暂时不做展示和统计。

## 语义

位置是**入账后采到的手机位置**：

- 付完款马上记账，这个位置约等于消费地点。用户看到通知后可能已经移动，定位也可能有延迟，这些偏差都接受。
- 位置记在 `recognitions` 上，账目通过 `entries.recognition_id` 关联拿到；`entries` 不加列。
- 只给**已入账**的识别写位置。被拒、出错或没确认的识别不留位置。

## 流程

```text
截屏 → POST /recognize → 循环 /step …（不变）
done：
  显示通知
  draft 非空（入账成功）→ 获取当前位置 → POST /location { draft, lat, lon }，忽略响应
  停止
```

先弹通知、再定位，用户看到「已记账」就可以离开了。

## 契约

### `done` 响应带回 draft

- 入账成功（包括幂等重复确认）时，`done` 的 `draft` 填这笔账的草稿。
- 其他 `done`（请求错误、入账失败）的 `draft` 仍为空字符串。

快捷指令只认「`draft` 非空就补采位置」这一条，要不要定位由服务器决定。这一条也是唯一的协议变化：`/recognize` 和 `/step` 的请求不变。

### `POST /location`

鉴权和其他接口相同。请求体是 JSON，三个字段都是文本，和 `/step` 一致：

```json
{ "draft": "<done 里的 draft>", "lat": "31.2304", "lon": "121.4737", "city": "上海市" }
```

- `city` 是系统在手机上解析出的城市（不经第三方），按地点看账时分组用；可以省略。去掉首尾空白后为空或超过 100 字就当作没有，不影响坐标写入。

- 解析 `draft` 拿到识别 ID。坐标必须两个都是数字，并且在范围内。
- 写入条件：该识别已有账目（`entries.recognition_id`），并且还没写过位置。**重复请求保留第一次的有效坐标**：

  ```sql
  UPDATE recognitions SET latitude = ?, longitude = ?
  WHERE draft_id = ? AND latitude IS NULL
    AND EXISTS (SELECT 1 FROM entries WHERE recognition_id = ?)
  ```

- 响应：写入或已存在返回 204；草稿或坐标无效返回 400；识别没入账返回 404。快捷指令不读响应。
- 日志只记固定原因（`坐标无效`、`未入账` 等），不带原始坐标。位置不发给 DeepSeek。

### 表结构

```sql
-- recognitions 加两列，WGS-84；只有已入账的识别会有
latitude  REAL CHECK (latitude  BETWEEN -90  AND 90),
longitude REAL CHECK (longitude BETWEEN -180 AND 180),
city      TEXT,   -- v11
CHECK ((latitude IS NULL) = (longitude IS NULL))
```

转换脚本 `deploy/db/10-location.sql` 加经纬度，`deploy/db/11-location-city.sql` 加城市；旧记录都为空。

### 快捷指令

在 `done` 分支的【显示通知】之后、【停止此快捷指令】之前，加上：

1. 【如果】`resp` 的 `draft` 有任何值
2. 　【获取当前位置】→【获取位置的详细信息】纬度、经度、城市
3. 　【获取 URL 内容】`base`/location，POST，JSON：`draft` / `lat` / `lon` / `city`

同步更新 `shortcuts/build.py` 和 `shortcuts/README.md`，然后重新生成并导入一次。

## 实机验证

- 拒绝定位权限、没信号：账已经记上，只是没有位置。
- 看到通知后立刻锁屏或切走：位置能不能补上（补不上也可以接受）。
- 首次运行时的授权弹窗出现在通知之后，会不会让人以为账还没记完。
- 实测一段时间的位置采集成功率。

坐标存的是 WGS-84，以后要在国内地图上展示，需要在展示端转换成 GCJ-02。

## 网页查看

`GET /api/recognitions/:id` 多返回 `location: {lat, lon, city} | null`（见 [Web 接口](web-api.md) 3.6），账目详情在截图下面显示城市（没有就显示「在地图中查看」），跳到 Apple 地图。

## 不做

- 网页手填记位置、在网页上修改位置、地名和逆地理编码、按地点统计（见 [头脑风暴](../brainstorm/location.md)）。
