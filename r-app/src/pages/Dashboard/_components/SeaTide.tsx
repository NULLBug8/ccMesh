import { motion, useReducedMotion } from "motion/react";

/**
 * 海水涨潮动效：代理运行时铺在卡片底部的水面。
 * 两层缓慢旋转的圆角方块在水线处制造起伏；尊重 prefers-reduced-motion（退化为静态水位）。
 */
export function SeaTide() {
  const reduce = useReducedMotion();
  return (
    <div
      aria-hidden
      className="pointer-events-none absolute inset-0 overflow-hidden rounded-[inherit]"
    >
      <motion.div
        className="absolute inset-x-0 bottom-0"
        initial={{ height: "0%" }}
        animate={{ height: "26%" }}
        transition={{ duration: 0.8, ease: "easeOut" }}
      >
        {/* 水体 */}
        <div className="absolute inset-x-0 bottom-0 top-3 bg-gradient-to-t from-primary/25 to-primary/8" />
        {/* 波浪层（旋转的圆角方块，仅顶缘露出水面形成起伏） */}
        {!reduce && (
          <>
            <motion.div
              className="absolute left-1/2 top-0 aspect-square w-[210%] -translate-x-1/2 -translate-y-[88%] rounded-[42%] bg-primary/20"
              animate={{ rotate: 360 }}
              transition={{ duration: 13, repeat: Infinity, ease: "linear" }}
            />
            <motion.div
              className="absolute left-1/2 top-0 aspect-square w-[210%] -translate-x-1/2 -translate-y-[85%] rounded-[45%] bg-primary/10"
              animate={{ rotate: -360 }}
              transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
            />
          </>
        )}
      </motion.div>
    </div>
  );
}
