import { useEffect, useState, type ReactNode } from "react";

/** 封面图：加载失败或没有地址时回退到 emoji 占位；children 叠加在图片之上。 */
export function Cover({
  src,
  emoji,
  children,
}: {
  src?: string | null;
  emoji: string;
  children?: ReactNode;
}) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);

  return (
    <div className="cover">
      {src && !failed ? (
        <img src={src} alt="" onError={() => setFailed(true)} />
      ) : (
        <div className="ph">{emoji}</div>
      )}
      {children}
    </div>
  );
}
