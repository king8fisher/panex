import { Box, Text } from 'ink';

interface StatusBarProps {
  focusMode: boolean;
  processName?: string;
  showShiftTabHint?: boolean;
}

export function StatusBar({ focusMode, processName, showShiftTabHint = true }: StatusBarProps) {
  if (focusMode && processName) {
    const shiftTabHint = showShiftTabHint ? 'Shift-Tab/' : '';
    return (
      <Box backgroundColor="green" width="100%">
        <Text bold color="black" backgroundColor="green">
          {' '}FOCUS: {processName} - Type to interact, [{shiftTabHint}Esc] to exit focus mode{' '}
        </Text>
      </Box>
    );
  }

  return (
    <Box backgroundColor="blue" width="100%">
      <Text bold color="black" backgroundColor="blue">
        {' '}[↑↓/jk] select  [Tab/Enter] focus  [r] restart  [A] restart All  [x] kill  [q] quit  [?] help{' '}
      </Text>
    </Box>
  );
}
