

import { Box, Typography } from "@mui/material";
import NumberFormat from "react-number-format";
import { Index, ResponseError, SearchResponse } from "../../utils/models";
import Loader from "../Loader";
import { ResultTable } from "./ResultTable";
import ErrorResponseDisplay from "../ResponseErrorDisplay";

function HitCount({searchResponse}: {searchResponse: SearchResponse}) {
  return (
    <Box>
      <Typography variant="body2" color="textSecondary">
        <NumberFormat
          displayType="text"
          value={searchResponse.num_hits}
          thousandSeparator=","
        />{" "}
        hits found in&nbsp;
        <NumberFormat
          decimalScale={2}
          displayType="text"
          value={searchResponse.elapsed_time_micros / 1000000}
          thousandSeparator=","
        />{" "}
        seconds
      </Typography>
    </Box>
  )
}

interface SearchResultProps {
  queryRunning: boolean;
  index: null | Index;
  searchResponse: null | SearchResponse;
  searchError: null | ResponseError;
}

export default function SearchResult(props: SearchResultProps) {
  if (props.queryRunning) {
    return <Loader />
  }
  if (props.searchError !== null) {
    return ErrorResponseDisplay(props.searchError);
  }
  if (props.searchResponse == null || props.index == null) {
    return <></>
  }
  return (
    <Box sx={{ pt: 1, flexGrow: '1', flexBasis: '0%', overflow: 'hidden'}} >
      <Box sx={{ height: '100%', flexDirection: 'column', flexGrow: 1, display: 'flex'}}>
        <Box sx={{ flexShrink: 0, display: 'flex', flexGrow: 0, flexBasis: 'auto' }}>
          <HitCount searchResponse={props.searchResponse} />
        </Box>
        <Box sx={{ pt: 2, flexGrow: 1, flexBasis: '0%', minHeight: 0, display: 'flex', flexDirection: 'column' }}>
          <ResultTable searchResponse={props.searchResponse} index={props.index} />
        </Box>
      </Box>
    </Box>
  );
}
