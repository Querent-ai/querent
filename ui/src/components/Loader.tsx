

import { Box, styled, keyframes } from '@mui/material';
import { ReactComponent as Logo } from '../assets/img/quiAssistIcon.svg';

const spin = keyframes`
from {
  transform: rotate(0deg);
}
to {
  transform: rotate(360deg);
}
`

const SpinningLogo = styled(Logo)`
height: 10vmin;
pointer-events: none;
fill: #CBD1DD;
animation: ${spin} infinite 5s linear;
`

export default function Loader() {
  return <Box
    display="flex"
    justifyContent="center"
    alignItems="center"
    minHeight="40vh"
    >
    <SpinningLogo></SpinningLogo>
  </Box>
}
