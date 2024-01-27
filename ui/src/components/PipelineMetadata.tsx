import {
  Typography,
  Table,
  TableContainer,
  TableHead,
  TableBody,
  TableRow,
  TableCell,
  Paper,
  TablePagination,
  styled,
  Card,
  CardContent,
  Grid,
  Icon,
} from '@mui/material';
import { useEffect, useMemo, useState } from 'react';
import { Client } from '../services/client';
import Loader from '../components/Loader';
import { ResponseError, PipelinesMetadata } from '../utils/models';
import {
  ViewUnderAppBarBox,
  FullBoxContainer,
  QBreadcrumbs,
} from '../components/LayoutUtils';
import ApiUrlFooter from '../components/ApiUrlFooter';
import ErrorResponseDisplay from '../components/ResponseErrorDisplay';

const PipelineInfoCard = styled(Card)(({ theme }) => ({
  backgroundColor: theme.palette.primary.main,
  color: theme.palette.common.white,
  marginBottom: theme.spacing(2),
}));

// StyledTableCell for custom styling
const StyledTableCell = styled(TableCell)(({ theme }) => ({
  backgroundColor: theme.palette.primary.main,
  color: theme.palette.common.white,
}));


function WorkflowsView() {
  const [loading, setLoading] = useState(false);
  const [responseError, setResponseError] =
    useState<ResponseError | null>(null);
  const [semanticServiceCounters, setSemanticServiceCounters] = useState({
    num_failed_pipelines: 0,
    num_running_pipelines: 0,
    num_successful_pipelines: 0,
  });
  const [pipelinesMetadata, setPipelinesMetadata] =
    useState<PipelinesMetadata | undefined>(undefined);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(5);
  const questerClient = useMemo(() => new Client(), []);

  const renderPipelineInfoCard = () => {
    if (responseError !== null) {
      return ErrorResponseDisplay(responseError);
    }
    if (loading) {
      return <Loader />;
    }

    return (
      <PipelineInfoCard>
        <CardContent>
          <Typography variant="h5" gutterBottom>
            Semantic Pipelines Overview
          </Typography>
          <Grid container spacing={2}>
            <Grid item xs={4}>
              <Icon color="inherit">analytics</Icon>
              <Typography>
                Running Pipelines: {semanticServiceCounters.num_running_pipelines}
              </Typography>
            </Grid>
            <Grid item xs={4}>
              <Icon color="inherit">check_circle</Icon>
              <Typography>
                Successful Pipelines: {semanticServiceCounters.num_successful_pipelines}
              </Typography>
            </Grid>
            <Grid item xs={4}>
              <Icon color="inherit">highlight_off</Icon>
              <Typography>
                Failed Pipelines: {semanticServiceCounters.num_failed_pipelines}
              </Typography>
            </Grid>
          </Grid>
        </CardContent>
      </PipelineInfoCard>
    );
  };

  const renderPipelinesTable = () => {
    if (pipelinesMetadata === undefined) {
      return <Loader />;
    }

    const emptyRows =
      rowsPerPage -
      Math.min(
        rowsPerPage,
        pipelinesMetadata.pipelines.length - page * rowsPerPage
      );

    return (
      <Card>
        <CardContent>
          <TableContainer component={Paper}>
            <Table>
              <TableHead>
                <TableRow>
                  <StyledTableCell>Pipeline ID</StyledTableCell>
                  <StyledTableCell>Name</StyledTableCell>
                  {/* Add more table headers based on your PipelinesMetadata structure */}
                </TableRow>
              </TableHead>
              <TableBody>
                {(rowsPerPage > 0
                  ? pipelinesMetadata.pipelines.slice(
                      page * rowsPerPage,
                      page * rowsPerPage + rowsPerPage
                    )
                  : pipelinesMetadata.pipelines
                ).map((pipeline) => (
                  <TableRow key={pipeline.pipeline_id}>
                    <TableCell>{pipeline.pipeline_id}</TableCell>
                    <TableCell>{pipeline.name}</TableCell>
                    <TableCell>{pipeline.import}</TableCell>
                    {/* Add more table cells based on your PipelinesMetadata structure */}
                  </TableRow>
                ))}
                {emptyRows > 0 && (
                  <TableRow style={{ height: 53 * emptyRows }}>
                    <TableCell colSpan={3} />
                  </TableRow>
                )}
              </TableBody>
            </Table>
            <TablePagination
              rowsPerPageOptions={[5, 10, 25]}
              component="div"
              count={pipelinesMetadata.pipelines.length}
              rowsPerPage={rowsPerPage}
              page={page}
              onPageChange={(_event, newPage) => setPage(newPage)}
              onRowsPerPageChange={(event) => {
                setRowsPerPage(parseInt(event.target.value, 10));
                setPage(0);
              }}
            />
          </TableContainer>
        </CardContent>
      </Card>
    );
  };

  useEffect(() => {
    setLoading(true);

    // Fetch Semantic Service Counters
    const fetchSemanticServiceCounters =
      questerClient.getSemanticServiceCounters();

    // Fetch Pipelines Metadata
    const fetchPipelinesMetadata =
      questerClient.getSemanticPipelinesMetadata();

    // Execute both requests in parallel
    Promise.all([fetchSemanticServiceCounters, fetchPipelinesMetadata])
      .then(([counters, metadata]) => {
        setResponseError(null);
        setLoading(false);
        setSemanticServiceCounters(counters);
        setPipelinesMetadata(metadata);
      })
      .catch((error) => {
        setLoading(false);
        setResponseError(error);
      });
  }, [questerClient]);

  return (
    <ViewUnderAppBarBox>
      <FullBoxContainer>
        <QBreadcrumbs aria-label="breadcrumb">
          <Typography color="text.primary">Semantic Pipelines</Typography>
        </QBreadcrumbs>
        {renderPipelineInfoCard()}
        {renderPipelinesTable()}
      </FullBoxContainer>
      {ApiUrlFooter('api/v1/semantics')}
    </ViewUnderAppBarBox>
  );
}

export default WorkflowsView;
