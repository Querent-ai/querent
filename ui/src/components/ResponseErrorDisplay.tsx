

import { Box } from '@mui/material';
import { ResponseError } from '../utils/models';
import SentimentVeryDissatisfiedIcon from '@mui/icons-material/SentimentVeryDissatisfied';

function renderMessage(error: ResponseError) {
  if (error.message !== null && error.message.includes('No search node available.')) {
    return <Box sx={{fontSize: 16, pt: 2, }}>
        Your cluster does not contain any search node. You need at least one search node.
      </Box>
  } else {
    return <>
      <Box sx={{fontSize: 16, pt: 2, }}>
        {error.status && <span>Status: {error.status}</span>}
      </Box>
      <Box sx={{ fontSize: 14, pt: 1, alignItems: 'center'}}>
        Error: {error.message}
      </Box>
    </>
  }
}

export default function ErrorResponseDisplay(error: ResponseError) {
  return <Box sx={{ pt: 2, display: 'flex', flexDirection: 'column', alignItems: 'center' }} >
    <SentimentVeryDissatisfiedIcon sx={{ fontSize: 60 }} />
    {renderMessage(error)}
  </Box>
}
