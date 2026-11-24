/**
 * WGS-84 → GCJ-02。Apple 地图在国内用高德底图，链接里的坐标按 GCJ-02 解读；
 * 库里存手机给的 WGS-84，只在拼地图链接时转换。国外原样返回。
 */
const A = 6378245.0;
const EE = 0.00669342162296594323;

/** 中国大陆的粗略范围：[北, 西, 南, 东]，含于 IN、不含于 OUT（台湾、越南北部、俄蒙、朝日边）。 */
const IN = [
  [49.2204, 79.4462, 42.8899, 96.33],
  [54.1415, 109.6872, 39.3742, 135.0002],
  [42.8899, 73.1246, 29.5297, 124.143255],
  [29.5297, 82.9684, 26.7186, 97.0352],
  [29.5297, 97.0253, 20.414096, 124.367395],
  [20.414096, 107.975793, 17.871542, 111.744104],
];
const OUT = [
  [25.398623, 119.921265, 21.785006, 122.497559],
  [22.284, 101.8652, 20.0988, 106.665],
  [21.5422, 106.4525, 20.4878, 108.051],
  [55.8175, 109.0323, 50.3257, 119.127],
  [55.8175, 127.4568, 49.5574, 137.0227],
  [44.8922, 131.2662, 42.5692, 137.0227],
];
const within =
  (lat: number, lon: number) =>
  ([n, w, s, e]: number[]) =>
    lat <= n && lat >= s && lon >= w && lon <= e;
const outOfChina = (lat: number, lon: number) => !IN.some(within(lat, lon)) || OUT.some(within(lat, lon));

function dLat(x: number, y: number) {
  let r = -100 + 2 * x + 3 * y + 0.2 * y * y + 0.1 * x * y + 0.2 * Math.sqrt(Math.abs(x));
  r += ((20 * Math.sin(6 * x * Math.PI) + 20 * Math.sin(2 * x * Math.PI)) * 2) / 3;
  r += ((20 * Math.sin(y * Math.PI) + 40 * Math.sin((y / 3) * Math.PI)) * 2) / 3;
  r += ((160 * Math.sin((y / 12) * Math.PI) + 320 * Math.sin((y * Math.PI) / 30)) * 2) / 3;
  return r;
}

function dLon(x: number, y: number) {
  let r = 300 + x + 2 * y + 0.1 * x * x + 0.1 * x * y + 0.1 * Math.sqrt(Math.abs(x));
  r += ((20 * Math.sin(6 * x * Math.PI) + 20 * Math.sin(2 * x * Math.PI)) * 2) / 3;
  r += ((20 * Math.sin(x * Math.PI) + 40 * Math.sin((x / 3) * Math.PI)) * 2) / 3;
  r += ((150 * Math.sin((x / 12) * Math.PI) + 300 * Math.sin((x / 30) * Math.PI)) * 2) / 3;
  return r;
}

export function toGcj(lat: number, lon: number): [number, number] {
  if (outOfChina(lat, lon)) return [lat, lon];
  const rad = (lat / 180) * Math.PI;
  const magic = 1 - EE * Math.sin(rad) ** 2;
  const sq = Math.sqrt(magic);
  const la = (dLat(lon - 105, lat - 35) * 180) / (((A * (1 - EE)) / (magic * sq)) * Math.PI);
  const lo = (dLon(lon - 105, lat - 35) * 180) / ((A / sq) * Math.cos(rad) * Math.PI);
  return [lat + la, lon + lo];
}
