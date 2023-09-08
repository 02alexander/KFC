use embedded_hal::{
    blocking::delay::DelayUs,
    digital::v2::{InputPin, OutputPin},
};

pub struct ButtonMatrix<OUTPIN, INPPIN, const COLS: usize, const ROWS: usize>
where
    OUTPIN: OutputPin,
    INPPIN: InputPin,
{
    pub rows: [OUTPIN; ROWS],
    pub cols: [INPPIN; COLS],
}

pub struct PressedIterator<'a, 'b, D, OUTPIN, INPPIN, const COLS: usize, const ROWS: usize>
where
    OUTPIN: OutputPin,
    INPPIN: InputPin,
{
    butmat: &'a mut ButtonMatrix<OUTPIN, INPPIN, COLS, ROWS>,
    delay: &'b mut D,
    cur_row: usize,
    cur_col: usize,
}

impl<'a, 'b, D: DelayUs<u16>, OUTPIN, INPPIN, const COLS: usize, const ROWS: usize> Iterator
    for PressedIterator<'a, 'b, D, OUTPIN, INPPIN, COLS, ROWS>
where
    OUTPIN: OutputPin,
    INPPIN: InputPin,
{
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_col >= COLS {
            self.cur_col = 0;
            self.cur_row += 1;
        }
        if self.cur_row >= ROWS {
            return None;
        };

        let row_pin = &mut self.butmat.rows[self.cur_row];
        let col_pin = &mut self.butmat.cols[self.cur_col];

        row_pin.set_high().ok()?;
        self.delay.delay_us(10);
        if col_pin.is_high().ok()? {
            row_pin.set_low().ok()?;
            Some((self.cur_row, self.cur_col))
        } else {
            row_pin.set_low().ok()?;
            None
        }
    }
}

impl<OUTPIN, INPPIN, const COLS: usize, const ROWS: usize> ButtonMatrix<OUTPIN, INPPIN, COLS, ROWS>
where
    OUTPIN: OutputPin,
    INPPIN: InputPin,
{
    pub fn scan(&mut self, delay: &mut impl DelayUs<u32>) -> Option<[[bool; COLS]; ROWS]> {
        let mut pressed = [[false; COLS]; ROWS];

        for (ri, row_pin) in self.rows.iter_mut().enumerate() {
            for (ci, col_pin) in self.cols.iter_mut().enumerate() {
                row_pin.set_high().ok()?;
                delay.delay_us(10);

                if col_pin.is_high().ok()? {
                    pressed[ri][ci] = true;
                }

                row_pin.set_low().ok()?;
            }
        }
        Some(pressed)
    }
}
