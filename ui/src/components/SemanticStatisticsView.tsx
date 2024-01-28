import React from 'react';
import { Card, CardContent, Typography, Box, Grid, IconButton } from '@mui/material';
import { IndexingStatistics } from '../utils/models';
import InsertDriveFileIcon from '@mui/icons-material/InsertDriveFile';
import TimelineIcon from '@mui/icons-material/Timeline';
import BatchPredictionIcon from '@mui/icons-material/BatchPrediction';
import PolylineRoundedIcon from '@mui/icons-material/PolylineRounded';
import CloseIcon from '@mui/icons-material/Close';

interface StatisticsViewProps {
  statistics: IndexingStatistics;
  pipelineId: string;
  onClose: () => void;
}

const StatisticsView: React.FC<StatisticsViewProps> = ({ pipelineId, statistics, onClose }) => {
  return (
    <Card className="statistics-sidebar">
      <Box display="flex" justifyContent="flex-end">
      </Box>
      <hr />
      <hr />
      <hr />
      <hr />
      <Typography variant="h6" sx={{ p: 2 }}>
          Statistics
        </Typography>
        <Typography variant="h5" gutterBottom>
          {pipelineId}
        </Typography>
      <IconButton className="close-button" onClick={onClose} size="small">
            <CloseIcon />
      </IconButton>
      <CardContent>
        <Grid container spacing={2}>
          <Grid item xs={12} sm={6} md={4} lg={3}>
            <Box display="flex" flexDirection="column" alignItems="center">
              <BatchPredictionIcon fontSize="large" color="primary" />
              <Typography>Total Batches: {statistics.total_batches}</Typography>
            </Box>
          </Grid>
          <Grid item xs={12} sm={6} md={4} lg={3}>
            <Box display="flex" flexDirection="column" alignItems="center">
              <InsertDriveFileIcon fontSize="large" color="primary" />
              <Typography>Total Docs: {statistics.total_docs}</Typography>
            </Box>
          </Grid>
          <Grid item xs={12} sm={6} md={4} lg={3}>
            <Box display="flex" flexDirection="column" alignItems="center">
              <TimelineIcon fontSize="large" color="primary" />
              <Typography>Total Graph Events: {statistics.total_graph_events}</Typography>
            </Box>
          </Grid>
          <Grid item xs={12} sm={6} md={4} lg={3}>
            <Box display="flex" flexDirection="column" alignItems="center">
              <PolylineRoundedIcon fontSize="large" color="primary" />
              <Typography>Total Vector Events: {statistics.total_vector_events}</Typography>
            </Box>
          </Grid>
        </Grid>
      </CardContent>
    </Card>
  );
};

export default StatisticsView;
