import { Box, Text, useStdout } from 'ink';
import { ScrollView, ScrollViewRef } from 'ink-scroll-view';
import { forwardRef, useImperativeHandle, useRef, useEffect, useState, useCallback } from 'react';
import { Scrollbar } from './Scrollbar';

interface OutputPanelProps {
  name: string;
  output: string;
  active: boolean;
  height?: number;
  autoScroll?: boolean;
  onAutoScrollChange?: (enabled: boolean) => void;
}

export interface OutputPanelRef {
  scrollBy: (delta: number) => void;
  scrollToTop: () => void;
  scrollToBottom: () => void;
  getScrollOffset: () => number;
  getContentHeight: () => number;
  getViewportHeight: () => number;
  isAtBottom: () => boolean;
}

export const OutputPanel = forwardRef<OutputPanelRef, OutputPanelProps>(
  function OutputPanel({ name, output, active, height, autoScroll = true, onAutoScrollChange }, ref) {
    const borderStyle = active ? 'double' : 'single';
    const lines = output.split('\n');
    const scrollRef = useRef<ScrollViewRef>(null);
    const { stdout } = useStdout();

    // Track scroll state for scrollbar
    const [scrollOffset, setScrollOffset] = useState(0);
    const [contentHeight, setContentHeight] = useState(0);
    const [viewportHeight, setViewportHeight] = useState(0);

    // Reset state synchronously when process changes (before render)
    const [prevName, setPrevName] = useState(name);
    if (name !== prevName) {
      setPrevName(name);
      setScrollOffset(0);
      setContentHeight(0);
      setViewportHeight(0);
    }

    // Handle terminal resize
    useEffect(() => {
      const handleResize = () => scrollRef.current?.remeasure();
      stdout?.on('resize', handleResize);
      return () => {
        stdout?.off('resize', handleResize);
      };
    }, [stdout]);

    // Check if at bottom with small tolerance
    const isAtBottom = useCallback(() => {
      if (!scrollRef.current) return true;
      const offset = scrollRef.current.getScrollOffset();
      const bottom = scrollRef.current.getBottomOffset();
      // Allow 1 line tolerance for rounding issues
      return offset >= bottom - 1;
    }, []);

    // Auto-scroll when content height changes (if enabled)
    const handleContentHeightChange = useCallback((newHeight: number) => {
      setContentHeight(newHeight);
      if (autoScroll && scrollRef.current) {
        // Use setTimeout to ensure layout is complete
        setTimeout(() => {
          scrollRef.current?.scrollToBottom();
        }, 0);
      }
    }, [autoScroll]);

    // Track scroll and update auto-scroll state
    const handleScroll = useCallback((offset: number) => {
      setScrollOffset(offset);

      // If user manually scrolled away from bottom, disable auto-scroll
      if (scrollRef.current) {
        const bottom = scrollRef.current.getBottomOffset();
        const atBottom = offset >= bottom - 1;

        if (!atBottom && autoScroll) {
          onAutoScrollChange?.(false);
        } else if (atBottom && !autoScroll) {
          onAutoScrollChange?.(true);
        }
      }
    }, [autoScroll, onAutoScrollChange]);

    // Expose scroll methods via ref
    useImperativeHandle(ref, () => ({
      scrollBy: (delta: number) => scrollRef.current?.scrollBy(delta),
      scrollToTop: () => scrollRef.current?.scrollToTop(),
      scrollToBottom: () => scrollRef.current?.scrollToBottom(),
      getScrollOffset: () => scrollRef.current?.getScrollOffset() ?? 0,
      getContentHeight: () => scrollRef.current?.getContentHeight() ?? 0,
      getViewportHeight: () => scrollRef.current?.getViewportHeight() ?? 0,
      isAtBottom,
    }));

    // Scrollbar height (same as content area)
    const scrollbarHeight = height ? height - 4 : 20;
    const hasScroll = contentHeight > viewportHeight;

    // Show pin indicator when auto-scroll is disabled (user scrolled up)
    const pinIndicator = !autoScroll && hasScroll ? ' ⍗' : '';

    return (
      <Box
        flexDirection="column"
        borderStyle={borderStyle}
        borderColor={active ? 'green' : 'gray'}
        flexGrow={1}
        height={height}
        paddingLeft={1}
      >
        <Box flexDirection="row" marginTop={0} height={height ? height - 2 : undefined}>
          <Box flexDirection="column" flexGrow={1}>
            <ScrollView
              key={name}
              ref={scrollRef}
              onScroll={handleScroll}
              onContentHeightChange={handleContentHeightChange}
              onViewportSizeChange={(layout) => setViewportHeight(layout.height)}
            >
              {lines.map((line, i) => (
                <Text key={i} wrap="truncate">{line}</Text>
              ))}
            </ScrollView>
          </Box>
          {hasScroll && (
            <Scrollbar
              scrollOffset={scrollOffset}
              contentHeight={contentHeight}
              viewportHeight={viewportHeight}
              height={scrollbarHeight}
            />
          )}
        </Box>
      </Box>
    );
  }
);
