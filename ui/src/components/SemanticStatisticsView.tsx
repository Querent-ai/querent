import React from 'react';
import { Card, CardContent, Typography, Box, Grid } from '@mui/material';
import { IndexingStatistics } from '../utils/models';
import InsertDriveFileIcon from '@mui/icons-material/InsertDriveFile';
import TimelineIcon from '@mui/icons-material/Timeline';
import BatchPredictionIcon from '@mui/icons-material/BatchPrediction';
import PolylineRoundedIcon from '@mui/icons-material/PolylineRounded';

interface StatisticsViewProps {
  statistics: IndexingStatistics;
}

const StatisticsView: React.FC<StatisticsViewProps> = ({ statistics }) => {
  return (
    <Card>
      <CardContent>
        <Typography variant="h5" gutterBottom>
          Indexing Statistics
        </Typography>
        <Grid container spacing={2}>
          <Grid item xs={6} sm={3}>
            <Box display="flex" alignItems="center">
              <BatchPredictionIcon fontSize="large" color="primary" />
              <Typography>Total Batches: {statistics.total_batches}</Typography>
            </Box>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Box display="flex" alignItems="center">
              <InsertDriveFileIcon fontSize="large" color="primary" />
              <Typography>Total Docs: {statistics.total_docs}</Typography>
            </Box>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Box display="flex" alignItems="center">
              <TimelineIcon fontSize="large" color="primary" />
              <Typography>Total Graph Events: {statistics.total_graph_events}</Typography>
            </Box>
          </Grid>
          <Grid item xs={6} sm={3}>
            <Box display="flex" alignItems="center">
              <PolylineRoundedIcon fontSize="large" color="primary" />
              <Typography>Total Vector Events: {statistics.total_vector_events}</Typography>
            </Box>
          </Grid>
          {/* Add more Grid items with icons and statistics as needed */}
        </Grid>
      </CardContent>
    </Card>
  );
};

export default StatisticsView;
