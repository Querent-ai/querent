

import MonacoEditor from 'react-monaco-editor';
import { useCallback } from "react";
import { EDITOR_THEME } from "../utils/theme";

export function JsonEditor({content, resizeOnMount}: {content: unknown, resizeOnMount: boolean}) {
  // Setting editor height based on lines height and count to stretch and fit its content.
  const onMount = useCallback((editor) => {
    if (!resizeOnMount) {
      return;
    }
    const editorElement = editor.getDomNode();

    if (!editorElement) {
      return;
    }

    // Weirdly enough, we have to wait a few ms to get the right height
    // from `editor.getContentHeight()`. If not, we sometimes end up with
    // a height > 7000px... and I don't know why.
    setTimeout(() => {
      const height = Math.min(800, editor.getContentHeight());
      editorElement.style.height = `${height}px`;
      editor.layout();
    }, 10);

  }, [resizeOnMount]);

  /* eslint-disable  @typescript-eslint/no-explicit-any */
  function beforeMount(monaco: any) {
    monaco.editor.defineTheme('quester-light', EDITOR_THEME);
  }

  return (
    <MonacoEditor
      language='json'
      value={JSON.stringify(content, null, 2)}
      editorWillMount={beforeMount}
      editorDidMount={onMount}
      options={{
        readOnly: true,
        fontFamily: 'monospace',
        overviewRulerBorder: false,
        overviewRulerLanes: 0,
        minimap: {
          enabled: false,
        },
        scrollbar: {
          alwaysConsumeMouseWheel: false,
        },
        renderLineHighlight: "gutter",
        fontSize: 12,
        fixedOverflowWidgets: true,
        scrollBeyondLastLine: false,
        automaticLayout: true,
        wordWrap: 'on',
        wrappingIndent: 'deepIndent',
      }}
      theme='quester-light'
    />
  )
}
