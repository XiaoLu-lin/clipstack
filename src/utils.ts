/** 相对时间：刚刚 / N 分钟前 / N 小时前 / 昨天 / M月D日 */
export function relativeTime(tsSecs: number): string {
  const diff = Date.now() - tsSecs * 1000;
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "刚刚";
  if (mins < 60) return `${mins} 分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} 小时前`;
  const d = new Date(tsSecs * 1000);
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const startOfThat = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  if (startOfToday - startOfThat === 86400000) return "昨天";
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

/** 时间分组标签：今天 / 昨天 / 更早 */
export function groupLabel(tsSecs: number): string {
  const d = new Date(tsSecs * 1000);
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const startOfThat = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const gap = startOfToday - startOfThat;
  if (gap <= 0) return "今天";
  if (gap === 86400000) return "昨天";
  return "更早";
}

export const KIND_LABEL: Record<string, string> = {
  text: "文本",
  link: "链接",
  color: "颜色",
  image: "图片",
  file: "文件",
};
