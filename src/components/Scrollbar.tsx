import { Box, Text } from 'ink';

interface ScrollbarProps {
  /** Current scroll offset */
  scrollOffset: number;
  /** Total content height */
  contentHeight: number;
  /** Visible viewport height */
  viewportHeight: number;
  /** Height of the scrollbar track (usually same as viewport) */
  height: number;
}

/**
 * A simple vertical scrollbar component.
 * Uses Unicode block characters to show scroll position.
 */
export function Scrollbar({
  scrollOffset,
  contentHeight,
  viewportHeight,
  height,
}: ScrollbarProps) {
  // Don't show scrollbar if content fits in viewport
  if (contentHeight <= viewportHeight) {
    return (
      <Box flexDirection="column" width={1}>
        {Array.from({ length: height }).map((_, i) => (
          <Text key={i} dimColor> </Text>
        ))}
      </Box>
    );
  }

  // Calculate thumb size and position
  const trackHeight = height;
  const thumbRatio = viewportHeight / contentHeight;
  const thumbHeight = Math.max(1, Math.round(trackHeight * thumbRatio));

  const maxScroll = contentHeight - viewportHeight;
  const scrollRatio = maxScroll > 0 ? scrollOffset / maxScroll : 0;
  const thumbPosition = Math.round((trackHeight - thumbHeight) * scrollRatio);

  // Build the scrollbar
  const lines: string[] = [];
  for (let i = 0; i < trackHeight; i++) {
    if (i >= thumbPosition && i < thumbPosition + thumbHeight) {
      lines.push('█'); // Thumb
    } else {
      lines.push('░'); // Track
    }
  }

  return (
    <Box flexDirection="column" width={1}>
      {lines.map((char, i) => (
        <Text key={i} dimColor={char === '░'}>{char}</Text>
      ))}
    </Box>
  );
}
