

import { Box, Button } from "@mui/material";
import { TimeRangeSelect } from './TimeRangeSelect';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import { SearchComponentProps } from "../utils/SearchComponentProps";

export function QueryEditorActionBar(props: SearchComponentProps) {
  const timestamp_field_name = props.index?.metadata.index_config.doc_mapping.timestamp_field;
  const shouldDisplayTimeRangeSelect = timestamp_field_name ?? false;
  return (
    <Box sx={{ display: 'flex'}}>
      <Box sx={{ flexGrow: 1 }}>
        <Button
          onClick={() => props.runSearch(props.searchRequest)}
          variant="contained"
          startIcon={<PlayArrowIcon />}
          disableElevation
          sx={{ flexGrow: 1}}
          disabled={props.queryRunning || !props.searchRequest.indexId}>
          Run
        </Button>
      </Box>
      { shouldDisplayTimeRangeSelect && <TimeRangeSelect
          timeRange={{
            startTimestamp:props.searchRequest.startTimestamp,
            endTimestamp:props.searchRequest.endTimestamp
          }}
          onUpdate={
            (timeRange)=>{
              props.runSearch({...props.searchRequest, ...timeRange});
            }
          }
          disabled={props.queryRunning || !props.searchRequest.indexId}
       />
      }
    </Box>
  )
}
