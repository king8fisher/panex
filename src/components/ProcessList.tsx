import { Box, Text, useStdout } from 'ink';
import { ScrollList, ScrollListRef } from 'ink-scroll-list';
import { forwardRef, useImperativeHandle, useRef, useEffect } from 'react';
import type { ProcessStatus } from '../types';

interface ProcessListProps {
  names: string[];
  selected: number;
  getStatus: (name: string) => ProcessStatus;
  active: boolean;
  height?: number;
}

export interface ProcessListRef {
  scrollBy: (delta: number) => void;
  scrollToTop: () => void;
  scrollToBottom: () => void;
}

export const ProcessList = forwardRef<ProcessListRef, ProcessListProps>(
  function ProcessList({ names, selected, getStatus, active, height }, ref) {
    const borderStyle = active ? 'double' : 'single';
    const listRef = useRef<ScrollListRef>(null);
    const { stdout } = useStdout();

    // Handle terminal resize
    useEffect(() => {
      const handleResize = () => listRef.current?.remeasure();
      stdout?.on('resize', handleResize);
      return () => {
        stdout?.off('resize', handleResize);
      };
    }, [stdout]);

    // Expose scroll methods via ref
    useImperativeHandle(ref, () => ({
      scrollBy: (delta: number) => listRef.current?.scrollBy(delta),
      scrollToTop: () => listRef.current?.scrollToTop(),
      scrollToBottom: () => listRef.current?.scrollToBottom(),
    }));

    return (
      <Box
        flexDirection="column"
        borderStyle={borderStyle}
        borderColor={active ? 'blue' : 'gray'}
        width={20}
        height={height}
        paddingX={1}
      >
        <Box flexDirection="column" marginTop={0} height={height ? height - 2 : undefined}>
          <ScrollList
            ref={listRef}
            selectedIndex={selected}
            scrollAlignment="auto"
          >
            {names.map((name, i) => {
              const status = getStatus(name);
              const statusIcon = status === 'running' ? '●' : status === 'error' ? '✗' : '○';
              const statusColor = status === 'running' ? 'green' : status === 'error' ? 'red' : 'gray';
              const isSelected = i === selected;

              return (
                <Box key={name} backgroundColor={isSelected ? 'blue' : undefined}>
                  <Text color={isSelected ? 'black' : undefined}>
                    {name}{' '}
                  </Text>
                  <Text color={isSelected ? 'black' : statusColor}>{statusIcon}</Text>
                </Box>
              );
            })}
          </ScrollList>
        </Box>
      </Box>
    );
  }
);
