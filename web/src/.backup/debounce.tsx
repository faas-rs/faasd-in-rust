import { useCallback, useRef } from "react";

export function useDebounce<T extends (...args: any[]) => any>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void { // Parameters<T> 获取泛型函数T参数类型的元组
  const timer = useRef<NodeJS.Timeout | null>(null); // 内置在@types/node的类型
  return useCallback(
    (...args: Parameters<T>) => {
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => {
        fn(...args);
      }, delay);
    },
    [fn, delay],
  );
}
