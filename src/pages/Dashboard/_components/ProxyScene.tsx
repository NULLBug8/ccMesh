import "./proxy-scene.css";

/**
 * 本地代理卡片背景：雪山 + 日落。
 * `running` 为真时太阳自山脊后升起、雪峰染暖色高山辉；为假时落回山后。
 * `dark` 按主题切换氛围元素：暗色显星点、亮色显飘云（颜色色板由 CSS 的 .dark 覆盖）。
 * 纯展示、不接收指针事件；尊重 prefers-reduced-motion（见 proxy-scene.css）。
 */
export function ProxyScene({
  running,
  dark = false,
}: {
  running: boolean;
  dark?: boolean;
}) {
  return (
    <div aria-hidden className={running ? "proxy-scene running" : "proxy-scene"}>
      <div className="glow" />
      <div className="sun" />
      {dark ? (
        <div className="stars" />
      ) : (
        <>
          <div className="cloud" />
          <div className="cloud cloud-1" />
        </>
      )}
      <div className="range range-far" />
      <div className="range range-mid" />
      <div className="range range-near" />
    </div>
  );
}
