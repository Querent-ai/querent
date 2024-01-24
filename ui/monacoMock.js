

import * as React from 'react';

// Mock MonocoEditor as the current jest setup does not work when Monaco JS files
// are loaded.
export default function MonacoEditor(props) {
  return <div>{props.value}</div>;
}

