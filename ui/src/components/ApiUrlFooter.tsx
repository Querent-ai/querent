

import { Box, styled, Typography, Button } from '@mui/material';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import { QUERENT_LIGHT_GREY } from '../utils/theme';

const Footer = styled(Box)`
display: flex;
height: 25px;
padding: 0px 5px;
position: absolute;
bottom: 0px;
font-size: 0.90em;
background-color: ${QUERENT_LIGHT_GREY};
opacity: 0.7;
`

export default function ApiUrlFooter(url: string) {
  const urlMaxLength = 80;
  const origin = process.env.NODE_ENV === 'development' ? 'http://localhost:7280' : window.location.origin;
  const completeUrl = `${origin}/${url}`;
  const isTooLong = completeUrl.length > urlMaxLength;
  return <Footer>
    <Typography sx={{ padding: '4px 5px', fontSize: '0.95em'}}>
      API URL:
    </Typography>
    <Button
      sx={{ fontSize: '0.93em', textTransform: 'inherit', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'clip' }}
      onClick={() => {
        if(window.isSecureContext){
          navigator.clipboard.writeText(completeUrl);
        } else {
          window.open(completeUrl, '_blank');
        }
      }}
      endIcon={<ContentCopyIcon />}
      size="small">
        {completeUrl.substring(0, urlMaxLength)}{isTooLong && "..."}
    </Button>
  </Footer>
}
