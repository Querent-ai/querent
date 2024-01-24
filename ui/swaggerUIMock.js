

import * as React from 'react';

// Mock SwaggerUI as the current jest setup does not work when Monaco JS files
// are loaded.
export default function SwaggerUI(props) {
  return <div>{props.url}</div>;
}

