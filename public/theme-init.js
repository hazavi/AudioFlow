try {
  const theme = JSON.parse(localStorage.getItem('audioflow-ui') || '{}').state?.theme || 'dark';
  const dark = theme === 'dark' || (theme === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.classList.toggle('dark', dark);
  document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
} catch { document.documentElement.classList.add('dark'); }
