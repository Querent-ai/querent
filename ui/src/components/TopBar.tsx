

import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import GitHubIcon from '@mui/icons-material/GitHub';
import { Box, IconButton, Link, styled, SvgIcon, Tooltip, Typography } from '@mui/material';
import { Discord } from '@styled-icons/fa-brands/Discord';
import { ReactComponent as Logo } from '../assets/img/querentMainLogo.svg';
import { Client } from '../services/client';
import { useEffect, useMemo, useState } from 'react';

const StyledAppBar = styled(AppBar)(({ theme })=>({
  zIndex: theme.zIndex.drawer + 1,
}));

// Update the Button's color prop options
declare module '@mui/material/AppBar' {
  interface AppBarPropsColorOverrides {
    neutral: true;
  }
}

const TopBar = () => {
  const [clusterId, setClusterId] = useState<string>("");
  const questerClient = useMemo(() => new Client(), []);

  useEffect(() => {
    questerClient.cluster().then(cluster => {
      setClusterId(cluster.cluster_id);
    });
  }, [])

  return (
    <StyledAppBar position="fixed" elevation={0} color="neutral">
      <Toolbar variant="dense">
        <Box sx={{ flexGrow: 1, p: 0, m: 0, display: 'flex', alignItems: 'center' }}>
          <Logo height='50px'></Logo>
          <Tooltip title="Cluster ID" placement="right">
            <Typography mx={2}>
              {clusterId}
            </Typography>
          </Tooltip>
        </Box>
        <Link href="https://querent.xyz/docs" target="_blank" sx={{ px: 2 }}>
            Docs
        </Link>
        <Link href="https://discord.gg/29T3DaeY" target="_blank">
          <IconButton size="large">
            <SvgIcon>
              <Discord />
            </SvgIcon>
          </IconButton>
        </Link>
        <Link href="https://github.com/querent-ai/quester" target="_blank">
          <IconButton size="large">
            <GitHubIcon />
          </IconButton>
        </Link>
      </Toolbar>
    </StyledAppBar>
  );
};

export default TopBar;
