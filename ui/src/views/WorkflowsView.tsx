import { Box, Typography } from '@mui/material';
import { useEffect, useMemo, useState } from 'react';
import { Client } from '../services/client';
import Loader from '../components/Loader';
import { ResponseError } from '../utils/models';
import { ViewUnderAppBarBox, FullBoxContainer, QBreadcrumbs } from '../components/LayoutUtils';
import ApiUrlFooter from '../components/ApiUrlFooter';
import ErrorResponseDisplay from '../components/ResponseErrorDisplay';

function WorkflowsView() {
  const [loading, setLoading] = useState(false);
  const [responseError, setResponseError] = useState<ResponseError | null>(null);
  const [semanticServiceCounters, setSemanticServiceCounters] = useState({
    num_failed_pipelines: 0,
    num_running_pipelines: 0,
    num_successful_pipelines: 0
  });
  const questerClient = useMemo(() => new Client(), []);

  const renderSemanticServiceCounters = () => {
    if (responseError !== null) {
      return ErrorResponseDisplay(responseError);
    }
    if (loading) {
      return <Loader />;
    }

    return (
      <Box>
        <Typography variant="h5" gutterBottom>
          Semantic Pipelines Overview
        </Typography>
        <Typography>
          Running Pipelines: {semanticServiceCounters.num_running_pipelines}
        </Typography>
        <Typography>
          Successful Pipelines: {semanticServiceCounters.num_successful_pipelines}
        </Typography>
        <Typography>
          Failed Pipelines: {semanticServiceCounters.num_failed_pipelines}
        </Typography>
      </Box>
    );
  };

  useEffect(() => {
    setLoading(true);
    questerClient.getSemanticServiceCounters().then(
      (counters) => {
        setResponseError(null);
        setLoading(false);
        setSemanticServiceCounters(counters);
      },
      (error) => {
        setLoading(false);
        setResponseError(error);
      }
    );
  }, [questerClient]);

  return (
    <ViewUnderAppBarBox>
      <FullBoxContainer>
        <QBreadcrumbs aria-label="breadcrumb">
          <Typography color="text.primary">Semantic Pipelines</Typography>
        </QBreadcrumbs>
        {renderSemanticServiceCounters()}
      </FullBoxContainer>
      {ApiUrlFooter('api/v1/semantics')}
    </ViewUnderAppBarBox>
  );
}

export default WorkflowsView;
