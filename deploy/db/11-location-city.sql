-- v10 → v11：补采位置时带上手机解析的城市，按地点看账用。
ALTER TABLE recognitions ADD COLUMN city TEXT;
