import { useRef, useState } from "react";

interface VirtualListProps<T> {
  items: T[];
  itemKey: (item: T) => string | number;
  rowHeight: number;
  height: number;
  overscan?: number;
  renderItem: (item: T) => React.ReactNode;
  className?: string;
}

/**
 * Fixed-row-height virtual scroller — no dependency added, since this project only
 * needs one virtualized list (the DLL translation review page's string table) and a
 * real library would be a lot of weight for that. Renders only the rows currently
 * within `height` (plus `overscan` rows of padding on each side) regardless of how
 * long `items` is, so a DLL with thousands of translatable strings doesn't force the
 * browser to lay out thousands of live `<input>` elements at once.
 */
export function VirtualList<T>({
  items,
  itemKey,
  rowHeight,
  height,
  overscan = 4,
  renderItem,
  className,
}: VirtualListProps<T>) {
  const [scrollTop, setScrollTop] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalHeight = items.length * rowHeight;
  const visibleCount = Math.ceil(height / rowHeight);
  const startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const endIndex = Math.min(items.length, startIndex + visibleCount + overscan * 2);
  const visibleItems = items.slice(startIndex, endIndex);

  return (
    <div
      ref={containerRef}
      className={className}
      style={{ height, overflowY: "auto", position: "relative" }}
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
    >
      <div style={{ height: totalHeight, position: "relative" }}>
        <div style={{ position: "absolute", top: startIndex * rowHeight, left: 0, right: 0 }}>
          {visibleItems.map((item) => (
            <div key={itemKey(item)} style={{ height: rowHeight, boxSizing: "border-box" }}>
              {renderItem(item)}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
