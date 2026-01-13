import { render } from 'ink';
import { createElement } from 'react';
import type { PanexConfig } from './types';
import { App } from './components/App';

export async function createTUI(config: PanexConfig): Promise<void> {
  const { waitUntilExit } = render(createElement(App, { config }));
  await waitUntilExit();
}
