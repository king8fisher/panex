import { Box, Text } from 'ink';

interface HelpPopupProps {
  visible: boolean;
}

export function HelpPopup({ visible }: HelpPopupProps) {
  if (!visible) return null;

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor="yellow"
      padding={1}
      position="absolute"
      marginLeft={10}
      marginTop={5}
    >
      <Text bold color="yellow"> Help </Text>
      <Text>{'\n'}Keyboard Shortcuts</Text>
      <Text>{'─'.repeat(18)}</Text>
      <Text>{'\n'}Navigation</Text>
      <Text>  ↑/↓ or j/k    Navigate process list</Text>
      <Text>  g/G           Scroll to top/bottom of output</Text>
      <Text>  PgUp/PgDn     Scroll output</Text>
      <Text>{'\n'}Process Control</Text>
      <Text>  Tab/Enter     Focus process (interactive mode)</Text>
      <Text>  Esc           Exit focus mode</Text>
      <Text>  r             Restart selected process</Text>
      <Text>  A             Restart all processes</Text>
      <Text>  x             Kill selected process</Text>
      <Text>{'\n'}General</Text>
      <Text>  ?             Toggle this help</Text>
      <Text>  q             Quit panex</Text>
      <Text>{'\n'}Press any key to close this help...</Text>
    </Box>
  );
}
