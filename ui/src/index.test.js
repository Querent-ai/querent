

import { render, screen } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import App from './views/App';

describe('App', function () {
  it('Should display side bar links', function () {
      render(<BrowserRouter><App /></BrowserRouter>);
      expect(screen.getByText(/Discover/)).toBeInTheDocument();
      expect(screen.getByText(/Query editor/)).toBeInTheDocument();
      expect(screen.getByText(/Admin/)).toBeInTheDocument();
  });
});
