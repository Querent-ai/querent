

import { useEffect, useRef, useState } from 'react';
import MonacoEditor from 'react-monaco-editor';
import * as monacoEditor from 'monaco-editor/esm/vs/editor/editor.api';
import { LANGUAGE_CONFIG, LanguageFeatures, createIndexCompletionProvider } from './config';
import { SearchComponentProps } from '../../utils/SearchComponentProps';
import { EDITOR_THEME } from '../../utils/theme';
import { Box } from '@mui/material';

const QUESTER_EDITOR_THEME_ID = 'quester-light';

function getLanguageId(indexId: string | null): string {
  if (indexId === null) {
    return '';
  }
  return `${indexId}-query-language`;
}

export function QueryEditor(props: SearchComponentProps) {
  const monacoRef = useRef<null | typeof monacoEditor>(null);
  const [languageId, setLanguageId] = useState<string>('');
  const runSearchRef = useRef(props.runSearch);
  const searchRequestRef = useRef(props.searchRequest);
  const defaultValue = props.searchRequest.query === null ? `// Select an index and type your query. Example: field_name:"phrase query"` : props.searchRequest.query;

  /* eslint-disable  @typescript-eslint/no-explicit-any */
  function handleEditorDidMount(editor: any, monaco: any) {
    monacoRef.current = monaco;
    editor.addAction({
      id: 'SEARCH',
      label: "Run search",
      keybindings: [
        monaco.KeyCode.F9,
        monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
      ],
      run: () => {
        runSearchRef.current(searchRequestRef.current);
      },
    })
  }

  useEffect(() => {
    const updatedLanguageId = getLanguageId(props.searchRequest.indexId);
    if (monacoRef.current !== null && updatedLanguageId !== '' && props.index !== null) {
      const monaco = monacoRef.current;
      if (!monaco.languages.getLanguages().some(({ id }: {id :string }) => id === updatedLanguageId)) {
        console.log('register language', updatedLanguageId);
        monaco.languages.register({'id': updatedLanguageId});
        monaco.languages.setMonarchTokensProvider(updatedLanguageId, LanguageFeatures())
        if (props.index != null) {
          monaco.languages.registerCompletionItemProvider(updatedLanguageId, createIndexCompletionProvider(props.index.metadata));
          monaco.languages.setLanguageConfiguration(
            updatedLanguageId,
            LANGUAGE_CONFIG,
          );
        }
      }
      setLanguageId(updatedLanguageId);
    }
  }, [monacoRef, props.index]);

  useEffect(() => {
    if (monacoRef.current !== null) {
      runSearchRef.current = props.runSearch;
    }
  }, [monacoRef, props.runSearch]);

  function handleEditorChange(value: any) {
    const updatedSearchRequest = Object.assign({}, props.searchRequest, {query: value});
    searchRequestRef.current = updatedSearchRequest;
    props.onSearchRequestUpdate(updatedSearchRequest);
  }

  function handleEditorWillMount(monaco: any) {
    monaco.editor.defineTheme(QUESTER_EDITOR_THEME_ID, EDITOR_THEME);
  }

  return (
    <Box sx={{ height: '100px', py: 1}} >
      <MonacoEditor
        editorWillMount={handleEditorWillMount}
        editorDidMount={handleEditorDidMount}
        onChange={handleEditorChange}
        language={languageId}
        value={defaultValue}
        options={{
          fontFamily: 'monospace',
          minimap: {
            enabled: false,
          },
          renderLineHighlight: "gutter",
          fontSize: 14,
          fixedOverflowWidgets: true,
          scrollBeyondLastLine: false,
      }}
      theme={QUESTER_EDITOR_THEME_ID}
      />
    </Box>
  );
}
